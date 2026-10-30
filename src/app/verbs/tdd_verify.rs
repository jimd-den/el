// `tdd verify`: close the run. The compiler must pass on the tree
// as it stands, and nothing may have moved since the run's last
// blessed state: test, production, and config all frozen, or the
// baseline is stale and verify refuses. On success the active
// marker is removed; the record stays as the run's history.

use super::super::error::Error;
use super::super::tdd::{self, Phase};
use super::super::{projection, Env, Exit, Report};
use super::inspect::prepare;
use super::tdd_run::{config_lock, current_locks, fs_err, load_active, phase_err, run_tool, save, test_locked, RUNS};
use crate::core::text::json::Json;

fn gate_failed(message: String) -> Error {
    Error { code: "EL_GATE_FAILED", exit: Exit::Gate, message }
}

pub(super) fn verify(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if record.phase != Phase::All {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "run is at {}, not all; verify follows all",
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
            test_locked(format!("test elements changed since all; the baseline is stale ({code})")),
        );
    }
    if let Err(code) = tdd::verify_frozen(&record.prod_baseline, &current_prod, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("production changed since all; the baseline is stale ({code})")),
        );
    }
    let current_config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(code) = tdd::verify_frozen(&record.config_lock, &current_config, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("config changed since all; the baseline is stale ({code})")),
        );
    }
    let argv = vec!["cargo".to_string(), "check".to_string(), "--all-targets".to_string()];
    let started = std::time::Instant::now();
    let finished = match run_tool(env, &argv) {
        Ok(finished) => finished,
        Err(error) => return Report::fail("tdd", error),
    };
    if finished.timed_out {
        return Report::fail(
            "tdd",
            gate_failed(format!("cargo check timed out after {}s", env.cargo_timeout)),
        );
    }
    let ms = started.elapsed().as_millis() as u64;
    if finished.code != 0 {
        return Report::fail(
            "tdd",
            gate_failed("cargo check failed; verify needs the tree compiling".to_string()),
        );
    }
    let moved = match tdd::transition(&record, Phase::Verify) {
        Ok(moved) => moved,
        Err(code) => return Report::fail("tdd", phase_err(code.to_string())),
    };
    if let Err(error) = save(env, &run, &moved) {
        return Report::fail("tdd", error);
    }
    if let Err(error) = env.fs.remove(&format!("{RUNS}/active")).map_err(fs_err) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.verify");
    report.lines = vec![
        format!("run {run}"),
        "phase verify".to_string(),
        format!("target {}", record.target),
        "run closed".to_string(),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("verify".to_string())),
        ("target".to_string(), Json::Str(record.target)),
    ]));
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("check".to_string())),
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
