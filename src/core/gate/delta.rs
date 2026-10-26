// The diagnostic delta: what this change introduced, fixed, or left alone.
// Identity is the fingerprint, never the line — so inserting a statement at
// the top of a body cannot report a phantom new error for every line below
// it. Duplicates collapse: rustc often emits the same diagnostic twice, and
// counting it twice would lie about the change.

use super::diagnostics::{Diagnostic, Level};
use super::Tier;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub new: Vec<Diagnostic>,
    pub fixed: Vec<Diagnostic>,
    pub existing: Vec<Diagnostic>,
}

pub fn delta(baseline: &[Diagnostic], current: &[Diagnostic]) -> Delta {
    let base = dedup(baseline);
    let curr = dedup(current);
    let base_fps: HashSet<String> = base.iter().map(|diag| diag.fingerprint()).collect();
    let curr_fps: HashSet<String> = curr.iter().map(|diag| diag.fingerprint()).collect();
    Delta {
        new: curr
            .iter()
            .filter(|diag| !base_fps.contains(&diag.fingerprint()))
            .cloned()
            .collect(),
        fixed: base
            .iter()
            .filter(|diag| !curr_fps.contains(&diag.fingerprint()))
            .cloned()
            .collect(),
        existing: curr
            .iter()
            .filter(|diag| base_fps.contains(&diag.fingerprint()))
            .cloned()
            .collect(),
    }
}

fn dedup(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for diag in diagnostics {
        if seen.insert(diag.fingerprint()) {
            kept.push(diag.clone());
        }
    }
    kept
}

/// Whether the tier passes on this delta. L0 always passes — the plan already
/// ran it. L1 blocks only on new errors; new warnings are reported, not
/// blocking. L2 blocks on any new diagnostic, because `-D warnings` is what
/// CI will run.
pub fn verdict(tier: Tier, delta: &Delta) -> bool {
    match tier {
        Tier::Model => true,
        Tier::Check => !delta.new.iter().any(|diag| diag.level == Level::Error),
        Tier::Strict => delta.new.is_empty(),
    }
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod delta_tests;
