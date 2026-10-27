// The three ways a staged transaction can end without committing: rejected by the
// compiler gate, previewed by `--dry-run`, or refused before staging. Each restores
// the old tree before returning, so a caller cannot forget the inverse.

use super::super::projection;
use super::super::report::{Error, Report};
use super::super::{error, Env, Exit};
use crate::core::gate::{Delta, Level, Tier};
use crate::core::history::entry::Entry;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;

#[path = "outcome/diagnostics.rs"]
mod diagnostics;

pub use diagnostics::lines as diagnostic_lines;

use diagnostics::gate_report;

/// The judged change, as one argument: its tier, its delta, and the catalog
/// that turns element ids into owner names for the diagnostic lines.
pub struct GateVerdict<'a> {
    pub tier: Tier,
    pub delta: &'a Delta,
    pub catalog: &'a Catalog,
    /// The elements this change edited, added or moved: the first actionable
    /// diagnostic is chosen from them before the rest.
    pub touched: &'a [ElId],
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
            diagnostics::plural(errors),
            verdict.delta.fixed.len(),
            verdict.delta.existing.len()
        )
    } else {
        format!(
            "change rejected — {} new warning{} ({} fixed, {} pre-existing)",
            new,
            diagnostics::plural(new),
            verdict.delta.fixed.len(),
            verdict.delta.existing.len()
        )
    };
    let mut report = Report::fail(
        cmd,
        Error { code: "gate.rejected", exit: Exit::Gate, message },
    );
    report.gate = Some(gate_report(verdict.tier, verdict.delta, "rejected", ms, verdict.touched));
    report.lines = diagnostics::lines(verdict.catalog, &verdict.delta.new, verdict.touched);
    report.notes = vec![format!(
        "el: no files changed; history unchanged (attempt recorded as seq {}, gate=rejected:{new})",
        entry.seq
    )];
    report
}

/// A dry run: the same shape as a rejection, always restored, always journaled as
/// `preview:<n>`. `accepted` is the verdict the real commit would have reached, and
/// the exit follows it (`FR-23`) — a preview that says 0 and then refuses the same
/// change on the next run is worse than no preview at all.
pub fn preview(
    env: &Env<'_>,
    old_tree: &Tree,
    entry: &mut Entry,
    cmd: &'static str,
    ms: u64,
    verdict: GateVerdict,
    accepted: bool,
) -> Report {
    if let Err(cause) = projection::restore(env, old_tree) {
        return Report::fail(cmd, cause);
    }
    let new = verdict.delta.new.len();
    entry.gate = format!("preview:{new}");
    let _ = env.journal.put_entry(entry);
    let _ = env.journal.clear_inflight();
    let mut report = if accepted {
        Report::new(cmd)
    } else {
        let errors = verdict
            .delta
            .new
            .iter()
            .filter(|diag| diag.level == Level::Error)
            .count();
        Report::fail(
            cmd,
            Error {
                code: "gate.would_reject",
                exit: Exit::Gate,
                message: format!(
                    "dry run: the change would be rejected — {} new error{} ({} fixed, {} pre-existing)",
                    errors,
                    diagnostics::plural(errors),
                    verdict.delta.fixed.len(),
                    verdict.delta.existing.len()
                ),
            },
        )
    };
    report.gate = Some(gate_report(verdict.tier, verdict.delta, "preview", ms, verdict.touched));
    report.lines = diagnostics::lines(verdict.catalog, &verdict.delta.new, verdict.touched);
    report.notes = vec![format!(
        "el: dry run — no files changed; history unchanged (preview recorded as seq {})",
        entry.seq
    )];
    report
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
