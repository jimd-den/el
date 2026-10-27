// `el check`: the compiler's answer about the tree as it stands, reported, not
// refused (ADR-0026). It runs the same tier the gate runs through the same parser,
// so a diagnostic `check` names and a diagnostic a commit refuses are one thing.
//
// It does not need an `el` crate: a plain Cargo crate gets an empty catalog and
// un-attributed diagnostics. That is what lets `make check` dogfood it on this
// repository, which has no `.el/` of its own.

use super::super::transaction;
use super::super::{error, load, Env, Exit, Report};
use crate::cli::request::Globals;
use crate::core::gate::Tier;
use crate::core::model::catalog::Catalog;
use crate::core::render::render;
use std::time::Instant;

pub fn check(env: &Env<'_>, globals: &Globals) -> Report {
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
    let diagnostics = match transaction::diagnostics(env, tier, &catalog, &projection) {
        Ok(diagnostics) => diagnostics,
        Err(error) => return Report::fail("check", error),
    };
    let ms = started.elapsed().as_millis() as u64;

    if tier == Tier::Model {
        let mut report = Report::new("check");
        report.gate = Some(transaction::gate_from_diagnostics(tier, &[], "ok", ms, &[]));
        report.notes = vec![format!(
            "el: check skipped the compiler (gate={})",
            tier.as_str()
        )];
        return report;
    }
    report(env, tier, &catalog, diagnostics, ms)
}

fn report(
    env: &Env<'_>,
    tier: Tier,
    catalog: &Catalog,
    diagnostics: Vec<crate::core::gate::Diagnostic>,
    ms: u64,
) -> Report {
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
        &diagnostics,
        if errors == 0 { "ok" } else { "errors" },
        ms,
        &[],
    ));
    report.lines = transaction::diagnostic_lines(catalog, &diagnostics, &[]);
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
    let _ = env;
    report
}

fn catalog(env: &Env<'_>) -> Result<Catalog, error::Error> {
    if env.fs.exists(".el/version").unwrap_or(false) {
        load::catalog(env)
    } else {
        Ok(Catalog::new())
    }
}
