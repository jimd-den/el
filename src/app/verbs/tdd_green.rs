// `tdd green`: prove the fix. The target test passes, the test lock
// is exactly as red froze it, and config is unchanged. Production
// changes are the point of green, so the green state snapshot
// replaces the start baseline for verify to check against. A changed
// test is `EL_TEST_LOCKED`; a target that still fails is
// `EL_GATE_FAILED`. The weakening check over every test is defense
// in depth: the model renders tests as plain `#[test] fn` with no
// attribute slot, so no el verb can express a skip today.

use super::super::error::Error;
use super::super::tdd::{self, Phase};
use super::super::{inspect, projection, Env, Exit, Report};
use super::inspect::prepare;
use super::tdd_run::{cargo_test, config_lock, current_locks, load_active, phase_err, save, target_passed};
use crate::core::model::kind::Kind;
use crate::core::text::json::Json;

fn gate_failed(message: String) -> Error {
    Error { code: "EL_GATE_FAILED", exit: Exit::Gate, message }
}

pub(super) fn green(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if record.phase != Phase::Red {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "run is at {}, not red; green follows red",
                super::super::tdd_record::phase_name(record.phase)
            )),
        );
    }
    let (catalog, projection) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(error) = projection::check_drift(env, &catalog, &env.crate_name) {
        return Report::fail("tdd", error);
    }
    let (current_tests, current_prod) = current_locks(&catalog, &projection);
    if let Err(code) = tdd::verify_frozen(&record.test_lock, &current_tests, "EL_TEST_LOCKED") {
        return Report::fail(
            "tdd",
            Error {
                code: "EL_TEST_LOCKED",
                exit: Exit::Conflict,
                message: format!("test elements changed since red; green runs the red tests ({code})"),
            },
        );
    }
    let current_config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(code) = tdd::verify_frozen(&record.config_lock, &current_config, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("config changed since red; green allows code changes only ({code})")),
        );
    }
    for element in &catalog.elements {
        if element.kind == Kind::Test
            && tdd::weakened(&inspect::source_of(&projection, element).unwrap_or_default())
        {
            return Report::fail(
                "tdd",
                Error {
                    code: "EL_TEST_LOCKED",
                    exit: Exit::Conflict,
                    message: format!("test {} is weakened; green runs the red tests", element.id),
                },
            );
        }
    }
    let bare = match super::test::filter_for(env, &record.target) {
        Ok(name) => name,
        Err(error) => return Report::fail("tdd", error),
    };
    let argv = ["cargo".to_string(), "test".to_string(), "--".to_string(), bare.clone()];
    let started = std::time::Instant::now();
    let finished = match cargo_test(env, Some(&bare)) {
        Ok(finished) => finished,
        Err(error) => return Report::fail("tdd", error),
    };
    if finished.timed_out {
        return Report::fail(
            "tdd",
            gate_failed(format!("cargo test timed out after {}s", env.cargo_timeout)),
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    if finished.code != 0 || !target_passed(&finished.stdout, &bare) {
        return Report::fail(
            "tdd",
            gate_failed("the target test did not pass; green needs it passing".to_string()),
        );
    }
    let mut moved = match tdd::transition(&record, Phase::Green) {
        Ok(moved) => moved,
        Err(code) => return Report::fail("tdd", phase_err(code.to_string())),
    };
    moved.prod_baseline = current_prod;
    if let Err(error) = save(env, &run, &moved) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.green");
    report.lines = vec![
        format!("run {run}"),
        "phase green".to_string(),
        format!("target {}", record.target),
        format!("test {}", bare),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("green".to_string())),
        ("target".to_string(), Json::Str(record.target)),
    ]));
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("test".to_string())),
        (
            "command".to_string(),
            Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect()),
        ),
        ("exit_code".to_string(), Json::Number(finished.code.to_string())),
        ("elapsed_ms".to_string(), Json::Number(ms.to_string())),
        ("result".to_string(), Json::Str("passed".to_string())),
    ]));
    report
}
