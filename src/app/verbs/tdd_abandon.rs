// `tdd abandon`: close a run early, deliberately. The active marker
// is removed; the record stays as the run's history, showing the
// phase it reached. A fresh `start` is allowed afterwards.

use super::super::tdd_record::phase_name;
use super::super::{Env, Report};
use super::tdd_run::{fs_err, load_active, RUNS};
use crate::core::text::json::Json;

pub(super) fn abandon(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(error) = env.fs.remove(&format!("{RUNS}/active")).map_err(fs_err) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.abandon");
    report.lines = vec![
        format!("run {run}"),
        format!("abandoned at {}", phase_name(record.phase)),
        format!("target {}", record.target),
    ];
    report.payload = Some(Json::Object(vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str(phase_name(record.phase).to_string())),
        ("target".to_string(), Json::Str(record.target)),
    ]));
    report
}
