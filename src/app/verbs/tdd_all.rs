// `tdd all`: the canonical suite, from impacted. No unrecorded
// mutation is allowed in: test, production, and config must all
// still match the record, so the suite runs the state impacted
// blessed, not a newer one.

use super::super::error::Error;
use super::super::tdd::{self, Phase};
use super::super::{projection, Env, Exit, Report};
use super::inspect::prepare;
use super::tdd_run::{cargo_test, config_lock, current_locks, load_active, phase_err, save, suite_passed, test_locked};
use crate::core::text::json::Json;

fn gate_failed(message: String) -> Error {
    Error { code: "EL_GATE_FAILED", exit: Exit::Gate, message }
}

pub(super) fn all(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if record.phase != Phase::Impacted {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "run is at {}, not impacted; all follows impacted",
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
            test_locked(format!("test elements changed since impacted; all runs the blessed state ({code})")),
        );
    }
    if let Err(code) = tdd::verify_frozen(&record.prod_baseline, &current_prod, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("production changed since impacted; all runs the blessed state ({code})")),
        );
    }
    let current_config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(code) = tdd::verify_frozen(&record.config_lock, &current_config, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("config changed since impacted; all runs the blessed state ({code})")),
        );
    }
    let argv = ["cargo".to_string(), "test".to_string()];
    let started = std::time::Instant::now();
    let finished = match cargo_test(env, None) {
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
    if finished.code != 0 || !suite_passed(&finished.stdout) {
        return Report::fail(
            "tdd",
            gate_failed("the full suite did not pass; all needs it passing".to_string()),
        );
    }
    let moved = match tdd::transition(&record, Phase::All) {
        Ok(moved) => moved,
        Err(code) => return Report::fail("tdd", phase_err(code.to_string())),
    };
    if let Err(error) = save(env, &run, &moved) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.all");
    report.lines = vec![
        format!("run {run}"),
        "phase all".to_string(),
        format!("target {}", record.target),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("all".to_string())),
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
