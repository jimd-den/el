// Shared plumbing for the `tdd` phases: the active run, its record,
// the locks, and the phase/invalid-red errors. No phase logic lives
// here, so a phase file can be read without the others.

use super::super::error::Error;
use super::super::tdd::Record;
use super::super::{inspect, Env, Exit};
use crate::core::history::sha256;
use crate::core::model::catalog::Catalog;
use crate::core::model::kind::Kind;
use crate::core::render::Projection;
use crate::core::text::json;
use crate::ports::tools::Finished;

pub(super) const RUNS: &str = ".el/runs";

pub(super) fn fs_err(error: impl std::fmt::Display) -> Error {
    Error { code: "env.tool_failed", exit: Exit::Env, message: error.to_string() }
}

pub(super) fn phase_err(message: String) -> Error {
    Error { code: "EL_TDD_PHASE", exit: Exit::Conflict, message }
}

pub(super) fn test_locked(message: String) -> Error {
    Error { code: "EL_TEST_LOCKED", exit: Exit::Conflict, message }
}

pub(super) fn invalid_red(message: String) -> Error {
    Error { code: "EL_INVALID_RED", exit: Exit::Conflict, message }
}

pub(super) fn run_id(env: &Env<'_>) -> String {
    format!("run-{}-{}", env.clock.now_unix(), std::process::id())
}

pub(super) fn record_path(run: &str) -> String {
    format!("{RUNS}/{run}/record")
}

pub(super) fn save(env: &Env<'_>, run: &str, record: &Record) -> Result<(), Error> {
    let text = json::write(&super::super::tdd_record::to_json(record));
    env.fs.write(&record_path(run), text.as_bytes()).map_err(fs_err)?;
    env.fs.write(&format!("{RUNS}/active"), run.as_bytes()).map_err(fs_err)?;
    Ok(())
}

pub(super) fn load_active(env: &Env<'_>) -> Result<(String, Record), Error> {
    let run = env
        .fs
        .read(&format!("{RUNS}/active"))
        .map_err(fs_err)?
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            phase_err("no active TDD run; start one with `el tdd start --target <test-id>`".to_string())
        })?;
    let bytes = env
        .fs
        .read(&record_path(&run))
        .map_err(fs_err)?
        .ok_or_else(|| phase_err(format!("active run {run} has no record; it cannot continue")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| phase_err(format!("active run {run} has a non-UTF-8 record")))?;
    let value = json::parse(&text)
        .ok_or_else(|| phase_err(format!("active run {run} has an unparsable record")))?;
    let record = super::super::tdd_record::from_json(&value)
        .map_err(|_| phase_err(format!("active run {run} has an invalid record")))?;
    Ok((run, record))
}

pub(super) fn config_lock(env: &Env<'_>) -> Result<Vec<(String, String)>, Error> {
    let mut lock = Vec::new();
    for path in ["el.policy", "Cargo.toml"] {
        if let Some(bytes) = env.fs.read(path).map_err(fs_err)? {
            lock.push((path.to_string(), format!("sha256:{}", sha256::hex(&sha256::digest(&bytes)))));
        }
    }
    Ok(lock)
}

/// One lock: element ids paired with their revision digests.
pub(super) type Lock = Vec<(String, String)>;

pub(super) fn current_locks(catalog: &Catalog, projection: &Projection) -> (Lock, Lock) {
    let mut tests = Vec::new();
    let mut prod = Vec::new();
    for element in &catalog.elements {
        let rev = inspect::rev_of(projection, element);
        if element.kind == Kind::Test {
            tests.push((element.id.to_string(), rev));
        } else {
            prod.push((element.id.to_string(), rev));
        }
    }
    (tests, prod)
}

/// A cargo run, with tool failures mapped and the timeout left to the
/// caller: a timeout is never a verdict, and each phase names what it
/// means there.
pub(super) fn run_tool(env: &Env<'_>, argv: &[String]) -> Result<Finished, Error> {
    match env.tools.run_timed(argv, None, env.cargo_timeout) {
        Ok(finished) => Ok(finished),
        Err(failure) => {
            if failure.missing {
                return Err(super::super::error::unavailable(failure.to_string()));
            }
            Err(Error { code: "env.tool_failed", exit: Exit::Env, message: failure.to_string() })
        }
    }
}

pub(super) fn cargo_test(env: &Env<'_>, filter: Option<&str>) -> Result<Finished, Error> {
    let mut argv = vec!["cargo".to_string(), "test".to_string()];
    if let Some(name) = filter {
        argv.push("--".to_string());
        argv.push(name.to_string());
    }
    run_tool(env, &argv)
}

/// The suite passed, and only the suite's pass counts: the run must
/// have completed with a summary and no test failed.
pub(super) fn suite_passed(stdout: &str) -> bool {
    if !stdout.contains("test result: ok") {
        return false;
    }
    !stdout.lines().any(|line| line.ends_with(" ... FAILED"))
}

/// The target passed, and only the target's pass counts: the run must
/// have completed with a summary, no test failed, and one ok line
/// names the target (bare, or module-qualified).
pub(super) fn target_passed(stdout: &str, bare: &str) -> bool {
    if !suite_passed(stdout) {
        return false;
    }
    stdout.lines().any(|line| {
        line.strip_prefix("test ")
            .and_then(|rest| rest.strip_suffix(" ... ok"))
            .is_some_and(|name| name == bare || name.ends_with(&format!("::{bare}")))
    })
}
