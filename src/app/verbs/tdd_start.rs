// `tdd start`: capture the baseline the later phases check against —
// the target test identity, the test lock, the production baseline,
// and the config lock, all as element revisions. One active run per
// crate; a second start is refused while one is active.

use super::super::error::Error;
use super::super::tdd::{Phase, Record};
use super::super::{inspect, projection, Env, Exit, Report};
use super::inspect::prepare;
use super::tdd_run::{config_lock, phase_err, run_id, save, RUNS};
use crate::core::model::kind::Kind;
use crate::core::text::json::Json;

pub(super) fn start(env: &Env<'_>, target: &str) -> Report {
    if env.fs.exists(&format!("{RUNS}/active")).unwrap_or(false) {
        return Report::fail(
            "tdd",
            phase_err("a TDD run is already active; finish it before starting another".to_string()),
        );
    }
    let (catalog, projection) = match prepare(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(error) = projection::check_drift(env, &catalog, &env.crate_name) {
        return Report::fail("tdd", error);
    }
    let id = match super::super::change::parse_id(target) {
        Ok(id) => id,
        Err(error) => return Report::fail("tdd", error),
    };
    let Some(element) = catalog.find(id) else {
        return Report::fail(
            "tdd",
            Error {
                code: "notfound.element",
                exit: Exit::NotFound,
                message: format!("{id} does not exist"),
            },
        );
    };
    if element.kind != Kind::Test {
        return Report::fail(
            "tdd",
            Error {
                code: "usage.bad_value",
                exit: Exit::Usage,
                message: format!("{id} is not a test; tdd start --target takes a test id"),
            },
        );
    }
    let mut test_lock = Vec::new();
    let mut prod_baseline = Vec::new();
    for element in &catalog.elements {
        let rev = inspect::rev_of(&projection, element);
        if element.kind == Kind::Test {
            test_lock.push((element.id.to_string(), rev));
        } else {
            prod_baseline.push((element.id.to_string(), rev));
        }
    }
    let config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    let record = Record {
        phase: Phase::Start,
        mode: "strict".to_string(),
        target: id.to_string(),
        test_lock,
        prod_baseline,
        config_lock: config,
    };
    let run = run_id(env);
    if let Err(error) = save(env, &run, &record) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.start");
    report.lines = vec![
        format!("run {run}"),
        "phase start".to_string(),
        format!("target {}", record.target),
        format!("tests locked {}", record.test_lock.len()),
        format!("production baseline {}", record.prod_baseline.len()),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("start".to_string())),
        ("target".to_string(), Json::Str(record.target)),
    ]));
    report
}
