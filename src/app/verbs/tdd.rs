// `el tdd`: the enforced strict cycle, start -> red -> green ->
// impacted -> all -> verify, plus abandon to close a run early.
// Each phase lives in its own file; this one only dispatches.

use super::super::{Env, Report};
use super::tdd_run::{load_active, phase_err};

pub fn tdd(env: &Env<'_>, phase: &str, target: Option<&str>) -> Report {
    match phase {
        "start" => super::tdd_start::start(env, target.unwrap_or_default()),
        "red" => super::tdd_red::red(env),
        "green" => super::tdd_green::green(env),
        "impacted" => super::tdd_impacted::impacted(env),
        "all" => super::tdd_all::all(env),
        "verify" => super::tdd_verify::verify(env),
        "abandon" => super::tdd_abandon::abandon(env),
        other => match load_active(env) {
            Err(error) => Report::fail("tdd", error),
            Ok((_run, _record)) => Report::fail(
                "tdd",
                phase_err(format!("tdd {other} is not built yet")),
            ),
        },
    }
}
