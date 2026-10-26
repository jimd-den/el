// The three ways a staged transaction can end without committing: rejected by the
// compiler gate, previewed by `--dry-run`, or refused before staging. Each restores
// the old tree before returning, so a caller cannot forget the inverse.

use super::super::projection;
use super::super::report::{Error, Gate, GateDiagnostic, Report};
use super::super::{error, Env, Exit};
use crate::core::gate::{Delta, Diagnostic, Level, Tier};
use crate::core::history::entry::Entry;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::plan::Violations;

/// The judged change, as one argument: its tier, its delta, and the catalog
/// that turns element ids into owner names for the diagnostic lines.
pub struct GateVerdict<'a> {
    pub tier: Tier,
    pub delta: &'a Delta,
    pub catalog: &'a Catalog,
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// One header line and one message line per diagnostic, in the GATE §4.3
/// shape. The header names the element and its owner when the diagnostic is
/// attributed; un-attributed diagnostics carry `(inexact)`.
pub fn diagnostic_lines(catalog: &Catalog, diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut lines = Vec::new();
    for diag in diagnostics {
        let mut header = format!(
            "  {}[{}] {}:{}",
            diag.level.as_str(),
            diag.code,
            diag.path,
            diag.line
        );
        if let Some(el) = diag.el {
            header.push_str(&format!("  {el}"));
            if let Some(owner) = diag.owner {
                let alias = catalog
                    .find(owner)
                    .and_then(|element| element.alias.as_ref())
                    .map(|alias| alias.to_string())
                    .unwrap_or_default();
                header.push_str(&format!("  (in {owner} {alias})"));
            }
        }
        if !diag.exact {
            header.push_str("  (inexact)");
        }
        lines.push(header);
        lines.push(format!("    {}", diag.message));
    }
    lines
}

fn to_gate_diagnostics(diagnostics: &[Diagnostic]) -> Vec<GateDiagnostic> {
    diagnostics
        .iter()
        .map(|diag| GateDiagnostic {
            code: diag.code.clone(),
            level: diag.level.as_str().to_string(),
            el: diag.el.map(|el| el.to_string()),
            owner: diag.owner.map(|owner| owner.to_string()),
            path: diag.path.clone(),
            line: diag.line,
            col: diag.col,
            message: diag.message.clone(),
            exact: diag.exact,
        })
        .collect()
}

fn gate_report(tier: Tier, delta: &Delta, verdict: &str, ms: u64) -> Gate {
    Gate {
        tier: tier.as_str().to_string(),
        new: delta.new.len(),
        fixed: delta.fixed.len(),
        existing: delta.existing.len(),
        verdict: verdict.to_string(),
        ms,
        diagnostics: to_gate_diagnostics(&delta.new),
    }
}

/// A change the gate refused: the tree restored, the attempt journaled as
/// `rejected:<n>`, the report carrying the new diagnostics and their counts.
pub fn reject(
    env: &Env<'_>,
    old_tree: &Tree,
    entry: &mut Entry,
    cmd: &'static str,
    ms: u64,
    verdict: GateVerdict,
) -> Report {
    if let Err(cause) = projection::restore(env, old_tree) {
        return Report::fail(cmd, cause);
    }
    let new = verdict.delta.new.len();
    let errors = verdict
        .delta
        .new
        .iter()
        .filter(|diag| diag.level == Level::Error)
        .count();
    entry.gate = format!("rejected:{new}");
    let _ = env.journal.put_entry(entry);
    let _ = env.journal.clear_inflight();
    let message = if errors > 0 {
        format!(
            "change rejected — {} new error{} ({} fixed, {} pre-existing)",
            errors,
            plural(errors),
            verdict.delta.fixed.len(),
            verdict.delta.existing.len()
        )
    } else {
        format!(
            "change rejected — {} new warning{} ({} fixed, {} pre-existing)",
            new,
            plural(new),
            verdict.delta.fixed.len(),
            verdict.delta.existing.len()
        )
    };
    let mut report = Report::fail(
        cmd,
        Error { code: "gate.rejected", exit: Exit::Gate, message },
    );
    report.gate = Some(gate_report(verdict.tier, verdict.delta, "rejected", ms));
    report.lines = diagnostic_lines(verdict.catalog, &verdict.delta.new);
    report.notes = vec![format!(
        "el: no files changed; history unchanged (attempt recorded as seq {}, gate=rejected:{new})",
        entry.seq
    )];
    report
}

/// A dry run that would be refused: the same shape as a rejection, but the
/// verdict is a preview and no error is raised.
pub fn preview(
    env: &Env<'_>,
    old_tree: &Tree,
    entry: &mut Entry,
    cmd: &'static str,
    ms: u64,
    verdict: GateVerdict,
) -> Report {
    if let Err(cause) = projection::restore(env, old_tree) {
        return Report::fail(cmd, cause);
    }
    let new = verdict.delta.new.len();
    entry.gate = format!("preview:{new}");
    let _ = env.journal.put_entry(entry);
    let _ = env.journal.clear_inflight();
    let mut report = Report::new(cmd);
    report.gate = Some(gate_report(verdict.tier, verdict.delta, "preview", ms));
    report.lines = diagnostic_lines(verdict.catalog, &verdict.delta.new);
    report
}

pub fn violation_error(violations: Violations) -> Error {
    let first = violations.first();
    Error {
        code: first.code.as_str(),
        exit: if first.code == crate::core::plan::Code::UnknownRef {
            Exit::NotFound
        } else {
            Exit::Violation
        },
        message: violations.to_string(),
    }
}

/// A failure after the entry was staged but before it committed. Restore the tree,
/// rewrite the attempt as rejected, and clear the marker; a committed head is left
/// alone, because a later entry must never be rewritten by an earlier failure.
pub fn abort(
    env: &Env<'_>,
    old_tree: &Tree,
    entry: &Entry,
    cmd: &'static str,
    cause: error::Error,
) -> Report {
    if let Err(restore) = projection::restore(env, old_tree) {
        return Report::fail(cmd, restore);
    }
    let committed = env.journal.head().ok().flatten().unwrap_or(0) >= entry.seq;
    if !committed {
        let mut failed = entry.clone();
        failed.gate = "rejected:0".to_string();
        let _ = env.journal.put_entry(&failed);
    }
    let _ = env.journal.clear_inflight();
    Report::fail(cmd, cause)
}
