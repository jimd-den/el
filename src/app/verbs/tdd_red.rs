// `tdd red`: prove the designated test fails by assertion, against
// unchanged production code and config, then freeze the test lock at
// red-time values for green to check against. A passing test, a test
// that cannot compile, or a timeout is refused with `EL_INVALID_RED`,
// never counted as evidence.

use super::super::tdd::{self, Phase};
use super::super::{inspect, projection, Env, Report};
use super::inspect::prepare;
use super::tdd_run::{cargo_test, config_lock, current_locks, invalid_red, load_active, phase_err, save};
use crate::core::text::json::Json;

pub(super) fn red(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if record.phase != Phase::Start {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "run is at {}, not start; red follows start",
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
    if let Err(code) = tdd::verify_frozen(&record.prod_baseline, &current_prod, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "production elements changed during red; red allows test changes only ({code})"
            )),
        );
    }
    let current_config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(code) = tdd::verify_frozen(&record.config_lock, &current_config, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("config changed during red; red allows test changes only ({code})")),
        );
    }
    let bare = match super::test::filter_for(env, &record.target) {
        Ok(name) => name,
        Err(error) => return Report::fail("tdd", error),
    };
    let target_id = match super::super::change::parse_id(&record.target) {
        Ok(id) => id,
        Err(error) => return Report::fail("tdd", error),
    };
    let Some(target) = catalog.find(target_id) else {
        return Report::fail(
            "tdd",
            invalid_red("the target test left the catalog during red; red cannot be proved".to_string()),
        );
    };
    if tdd::weakened(&inspect::source_of(&projection, target).unwrap_or_default()) {
        return Report::fail(
            "tdd",
            invalid_red("the target test is weakened (#[ignore]); a skipped test is not red".to_string()),
        );
    }
    let argv = ["cargo".to_string(), "test".to_string(), "--".to_string(), bare.clone()];
    let started = std::time::Instant::now();
    let finished = match cargo_test(env, Some(&bare)) {
        Ok(finished) => finished,
        Err(error) => return Report::fail("tdd", error),
    };
    if finished.timed_out {
        return Report::fail(
            "tdd",
            invalid_red(format!(
                "cargo test timed out after {}s; a timeout is not red evidence",
                env.cargo_timeout
            )),
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    match tdd::classify_red(&finished.stdout, &bare, false) {
        tdd::RedVerdict::Assertion => {}
        tdd::RedVerdict::Setup => {
            return Report::fail(
                "tdd",
                invalid_red(
                    "the target test did not fail by assertion; a setup error is not red".to_string(),
                ),
            );
        }
        tdd::RedVerdict::Passing => {
            return Report::fail(
                "tdd",
                invalid_red("the target test already passes; red needs a failing test".to_string()),
            );
        }
    }
    let mut moved = match tdd::transition(&record, Phase::Red) {
        Ok(moved) => moved,
        Err(code) => return Report::fail("tdd", phase_err(code.to_string())),
    };
    moved.test_lock = current_tests;
    if let Err(error) = save(env, &run, &moved) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.red");
    report.lines = vec![
        format!("run {run}"),
        "phase red".to_string(),
        format!("target {}", record.target),
        format!("test {}", bare),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("red".to_string())),
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
        ("result".to_string(), Json::Str("failed".to_string())),
    ]));
    report
}
