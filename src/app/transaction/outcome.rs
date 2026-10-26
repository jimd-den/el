// The three ways a staged transaction can end without committing: rejected by the
// lint gate, previewed by `--dry-run`, or refused before staging. Each restores
// the old tree before returning, so a caller cannot forget the inverse.

use super::super::projection;
use super::super::report::{Error, Gate, Report};
use super::super::{error, Env, Exit};
use crate::core::history::entry::Entry;
use crate::core::history::tree::Tree;
use crate::core::plan::Violations;

pub fn reject(env: &Env<'_>, old_tree: &Tree, entry: &mut Entry, cmd: &'static str, ms: u64) -> Report {
    if let Err(cause) = projection::restore(env, old_tree) {
        return Report::fail(cmd, cause);
    }
    entry.gate = "rejected:1".to_string();
    let _ = env.journal.put_entry(entry);
    let _ = env.journal.clear_inflight();
    let mut report = Report::fail(
        cmd,
        Error {
            code: "gate.rejected",
            exit: Exit::Gate,
            message: "the lint gate rejected the change; the tree was restored".to_string(),
        },
    );
    report.gate = Some(gate(ms, "strict", "rejected", 1));
    report
}

pub fn preview(env: &Env<'_>, old_tree: &Tree, entry: &mut Entry, cmd: &'static str, ms: u64) -> Report {
    if let Err(cause) = projection::restore(env, old_tree) {
        return Report::fail(cmd, cause);
    }
    entry.gate = "preview:0".to_string();
    let _ = env.journal.put_entry(entry);
    let _ = env.journal.clear_inflight();
    let mut report = Report::new(cmd);
    report.gate = Some(gate(ms, "model", "preview", 0));
    report
}

fn gate(ms: u64, tier: &str, verdict: &str, new: usize) -> Gate {
    Gate {
        tier: tier.to_string(),
        new,
        fixed: 0,
        existing: 0,
        verdict: verdict.to_string(),
        ms,
    }
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
