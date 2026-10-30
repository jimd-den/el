// `tdd impacted`: run the tests the green change touched — every
// test whose statements mention a production element that changed
// since green, plus the locked target test, which is always re-run.
// When a changed element has no name to scan for (an impl block),
// impact is unavailable and the full suite runs instead, recorded as
// `fallback: all`. Production-only diffs are the point of impacted,
// so the blessed baseline moves to the post-impacted state.

use super::super::error::Error;
use super::super::tdd_impact::impacted_tests;
use super::super::tdd::{self, Phase};
use super::super::{inspect, projection, Env, Exit, Report};
use super::inspect::prepare;
use super::tdd_run::{
    cargo_test, config_lock, current_locks, load_active, phase_err, save, suite_passed,
    target_passed, test_locked,
};
use crate::core::model::kind::Kind;
use crate::core::text::json::Json;

fn gate_failed(message: String) -> Error {
    Error { code: "EL_GATE_FAILED", exit: Exit::Gate, message }
}

pub(super) fn impacted(env: &Env<'_>) -> Report {
    let (run, record) = match load_active(env) {
        Ok(loaded) => loaded,
        Err(error) => return Report::fail("tdd", error),
    };
    if record.phase != Phase::Green {
        return Report::fail(
            "tdd",
            phase_err(format!(
                "run is at {}, not green; impacted follows green",
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
            test_locked(format!("test elements changed since green; impacted runs the green tests ({code})")),
        );
    }
    let current_config = match config_lock(env) {
        Ok(lock) => lock,
        Err(error) => return Report::fail("tdd", error),
    };
    if let Err(code) = tdd::verify_frozen(&record.config_lock, &current_config, "EL_TDD_PHASE") {
        return Report::fail(
            "tdd",
            phase_err(format!("config changed since green; impacted allows code changes only ({code})")),
        );
    }
    for element in &catalog.elements {
        if element.kind == Kind::Test
            && tdd::weakened(&inspect::source_of(&projection, element).unwrap_or_default())
        {
            return Report::fail(
                "tdd",
                test_locked(format!("test {} is weakened; impacted runs the green tests", element.id)),
            );
        }
    }
    let changed: Vec<(String, Option<String>)> = current_prod
        .iter()
        .filter(|(id, rev)| {
            record.prod_baseline.iter().find(|(base_id, _)| base_id == id)
                .is_none_or(|(_, base_rev)| base_rev != rev)
        })
        .filter_map(|(id, _)| {
            catalog.elements.iter().find(|element| &element.id.to_string() == id)
                .map(|element| (id.clone(), element.name.clone()))
        })
        .collect();
    let test_stmts: Vec<(String, Vec<String>)> = catalog
        .elements
        .iter()
        .filter(|element| element.kind == Kind::Test)
        .map(|element| {
            let stmts = catalog
                .statements
                .iter()
                .filter(|stmt| stmt.owner == element.id)
                .map(|stmt| stmt.text.clone())
                .collect();
            (element.id.to_string(), stmts)
        })
        .collect();
    let started = std::time::Instant::now();
    let mut commands: Vec<Vec<String>> = Vec::new();
    let fallback = match impacted_tests(&changed, &test_stmts, &record.target) {
        None => true,
        Some(ids) => {
            let mut ok = true;
            for id in &ids {
                let bare = match super::test::filter_for(env, id) {
                    Ok(name) => name,
                    Err(error) => return Report::fail("tdd", error),
                };
                commands.push(vec![
                    "cargo".to_string(),
                    "test".to_string(),
                    "--".to_string(),
                    bare.clone(),
                ]);
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
                if finished.code != 0 || !target_passed(&finished.stdout, &bare) {
                    ok = false;
                    break;
                }
            }
            if !ok {
                return Report::fail(
                    "tdd",
                    gate_failed("an impacted test did not pass; impacted needs them passing".to_string()),
                );
            }
            false
        }
    };
    if fallback {
        commands.push(vec!["cargo".to_string(), "test".to_string()]);
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
        if finished.code != 0 || !suite_passed(&finished.stdout) {
            return Report::fail(
                "tdd",
                gate_failed("the full suite did not pass; impacted needs it passing".to_string()),
            );
        }
    }
    let ms = started.elapsed().as_millis() as u64;
    let mut moved = match tdd::transition(&record, Phase::Impacted) {
        Ok(moved) => moved,
        Err(code) => return Report::fail("tdd", phase_err(code.to_string())),
    };
    moved.prod_baseline = current_prod;
    if let Err(error) = save(env, &run, &moved) {
        return Report::fail("tdd", error);
    }
    let mut report = Report::new("tdd.impacted");
    report.lines = vec![
        format!("run {run}"),
        "phase impacted".to_string(),
        format!("target {}", record.target),
        format!("fallback {}", if fallback { "all" } else { "none" }),
    ];
    let mut fields = vec![
        ("run".to_string(), Json::Str(run)),
        ("phase".to_string(), Json::Str("impacted".to_string())),
        ("target".to_string(), Json::Str(record.target)),
    ];
    if fallback {
        fields.push(("fallback".to_string(), Json::Str("all".to_string())));
    }
    report.payload = Some(Json::Object(fields));
    report.evidence = Some(Json::Object(vec![
        ("kind".to_string(), Json::Str("test".to_string())),
        (
            "commands".to_string(),
            Json::Array(
                commands
                    .iter()
                    .map(|argv| Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect()))
                    .collect(),
            ),
        ),
        ("elapsed_ms".to_string(), Json::Number(ms.to_string())),
        ("result".to_string(), Json::Str("passed".to_string())),
    ]));
    report
}
