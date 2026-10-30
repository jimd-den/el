// `el check`: the compiler's answer about the tree as it stands, reported, not
// refused (ADR-0026). It runs the same tier the gate runs through the same parser,
// so a diagnostic `check` names and a diagnostic a commit refuses are one thing.
//
// It does not need an `el` crate: a plain Cargo crate gets an empty catalog and
// un-attributed diagnostics. That is what lets `make check` dogfood it on this
// repository, which has no `.el/` of its own.

use super::super::transaction;
use super::super::{error, load, Env, Exit, Report};
use super::scope::evidence_scope;
use crate::cli::request::Globals;
use crate::core::gate::{Parsed, Tier};
use crate::core::model::catalog::Catalog;
use crate::core::render::render;
use crate::core::text::json::Json;
use std::time::Instant;

pub fn check(env: &Env<'_>, globals: &Globals, scope: Option<&str>) -> Report {
    if !env.fs.exists("Cargo.toml").unwrap_or(false) {
        return Report::fail(
            "check",
            error::Error {
                code: "env.not_a_project",
                exit: Exit::Env,
                message: format!("{} has no Cargo.toml", env.root),
            },
        );
    }
    let policy = match load::policy(env) {
        Ok(policy) => policy,
        Err(error) => return Report::fail("check", error),
    };
    let tier = match transaction::select(&policy.gate, globals.fast, globals.strict) {
        Ok(tier) => tier,
        Err(error) => return Report::fail("check", error),
    };
    let catalog = match catalog(env) {
        Ok(catalog) => catalog,
        Err(error) => return Report::fail("check", error),
    };
    let projection = render(&catalog, &env.crate_name);
    let started = Instant::now();
    let parsed = match transaction::diagnostics(env, tier, &catalog, &projection) {
        Ok(parsed) => parsed,
        Err(error) => return Report::fail("check", remap_missing_tool(error)),
    };
    let ms = started.elapsed().as_millis() as u64;

    if tier == Tier::Model {
        let mut report = Report::new("check");
        report.gate = Some(transaction::gate_from_diagnostics(tier, &[], "ok", ms, &[]));
        report.notes = vec![format!(
            "el: check skipped the compiler (gate={})",
            tier.as_str()
        )];
        report.payload = Some(component_statuses("not_run"));
        return report;
    }
    report(env, tier, &catalog, parsed, scope, ms)
}

/// The gate names a missing tool `env.missing_tool`; a reporting verb
/// names it `EL_ENV_UNAVAILABLE`, so a missing cargo is never read as
/// a verdict about the code. The gate keeps its own code: commits do
/// not go through this mapping.
fn remap_missing_tool(error: error::Error) -> error::Error {
    if error.code == "env.missing_tool" {
        error::unavailable(error.message)
    } else {
        error
    }
}

/// The four component statuses. A verb reports its own and leaves the
/// others `not_run`: a check that passed says nothing about tests.
fn component_statuses(check: &str) -> Json {
    Json::Object(vec![
        ("check".to_string(), Json::Str(check.to_string())),
        ("test".to_string(), Json::Str("not_run".to_string())),
        ("fmt".to_string(), Json::Str("not_run".to_string())),
        ("lint".to_string(), Json::Str("not_run".to_string())),
    ])
}

fn report(
    env: &Env<'_>,
    tier: Tier,
    catalog: &Catalog,
    parsed: Parsed,
    scope: Option<&str>,
    ms: u64,
) -> Report {
    let diagnostics = &parsed.diagnostics;
    let errors = diagnostics
        .iter()
        .filter(|diag| diag.level == crate::core::gate::Level::Error)
        .count();
    let mut report = if errors == 0 {
        Report::new("check")
    } else {
        Report::fail(
            "check",
            error::Error {
                code: "gate.failed",
                exit: Exit::Gate,
                message: format!(
                    "check found {errors} error{}",
                    if errors == 1 { "" } else { "s" }
                ),
            },
        )
    };
    report.gate = Some(transaction::gate_from_diagnostics(
        tier,
        diagnostics,
        if errors == 0 { "ok" } else { "errors" },
        ms,
        &[],
    ));
    report.lines = transaction::diagnostic_lines(catalog, diagnostics, &[]);
    let label = match tier {
        Tier::Model => "L0 model",
        Tier::Check => "L1 check",
        Tier::Strict => "L2 strict",
    };
    report.notes = vec![format!(
        "el: check {} ({label}, {} diagnostics, {ms} ms)",
        if errors == 0 { "ok" } else { "failed" },
        diagnostics.len(),
    )];
    let result = if errors == 0 { "passed" } else { "failed" };
    report.payload = Some(Json::Object(vec![
        ("check".to_string(), Json::Str(result.to_string())),
        ("test".to_string(), Json::Str("not_run".to_string())),
        ("fmt".to_string(), Json::Str("not_run".to_string())),
        ("lint".to_string(), Json::Str("not_run".to_string())),
        (
            "non_json_lines".to_string(),
            Json::Array(
                parsed
                    .non_json
                    .iter()
                    .take(20)
                    .map(|line| Json::Str(line.clone()))
                    .collect(),
            ),
        ),
    ]));
    let commands = transaction::tier_argv(tier);
    let mut evidence = vec![
        ("kind".to_string(), Json::Str("check".to_string())),
        (
            "command".to_string(),
            Json::Array(
                commands[0].iter().map(|arg| Json::Str(arg.clone())).collect(),
            ),
        ),
    ];
    if commands.len() > 1 {
        evidence.push((
            "commands".to_string(),
            Json::Array(
                commands
                    .iter()
                    .map(|argv| {
                        Json::Array(argv.iter().map(|arg| Json::Str(arg.clone())).collect())
                    })
                    .collect(),
            ),
        ));
    }
    evidence.push((
        "scope".to_string(),
        evidence_scope(env, scope, &["all-targets"], &commands[0]),
    ));
    evidence.push((
        "exit_code".to_string(),
        parsed.exit_code.map(|code| Json::Number(code.to_string())).unwrap_or(Json::Null),
    ));
    evidence.push(("elapsed_ms".to_string(), Json::Number(ms.to_string())));
    evidence.push(("result".to_string(), Json::Str(result.to_string())));
    report.evidence = Some(Json::Object(evidence));
    report
}

fn catalog(env: &Env<'_>) -> Result<Catalog, error::Error> {
    if env.fs.exists(".el/version").unwrap_or(false) {
        load::catalog(env)
    } else {
        Ok(Catalog::new())
    }
}
