// Delta and verdict: new, fixed, existing — and which tier blocks on what.

use super::{delta, verdict, Delta};
use crate::core::gate::Tier;
use crate::core::gate::diagnostics::{Diagnostic, Level};
use crate::core::model::id::ElId;

fn error(code: &str, id: u32, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        level: Level::Error,
        el: Some(ElId(id)),
        owner: None,
        path: "src/main.rs".to_string(),
        line: 3,
        col: 1,
        message: message.to_string(),
        exact: true,
    }
}

fn warning(code: &str, id: u32, message: &str) -> Diagnostic {
    Diagnostic {
        level: Level::Warning,
        ..error(code, id, message)
    }
}

#[test]
fn a_delta_names_new_fixed_and_existing() {
    let old = error("E0308", 7, "mismatched types");
    let fresh = error("E0425", 9, "unresolved name");

    let baseline = [old.clone()];
    let current = [old, fresh.clone()];
    let found = delta(&baseline, &current);

    assert_eq!(found.new, vec![fresh]);
    assert!(found.fixed.is_empty());
    assert_eq!(found.existing.len(), 1);
}

#[test]
fn a_delta_reports_a_fix() {
    let kept = error("E0308", 7, "mismatched types");
    let gone = error("E0425", 9, "unresolved name");

    let found = delta(&[kept.clone(), gone.clone()], &[kept]);

    assert_eq!(found.fixed, vec![gone]);
    assert!(found.new.is_empty());
}

#[test]
fn a_delta_collapses_duplicate_frames_to_one_diagnostic() {
    let same = error("E0308", 7, "mismatched types");

    let found = delta(&[], &[same.clone(), same]);

    assert_eq!(found.new.len(), 1);
}

#[test]
fn a_fallback_diagnostic_that_moves_lines_is_existing_not_new() {
    let baseline = Diagnostic {
        code: "E0601".to_string(),
        level: Level::Error,
        el: None,
        owner: None,
        path: "src/main.rs".to_string(),
        line: 5,
        col: 1,
        message: "`main` function not found in crate `hello`".to_string(),
        exact: false,
    };
    let moved = Diagnostic { line: 9, ..baseline.clone() };

    let found = delta(&[baseline], &[moved]);

    assert!(found.new.is_empty());
    assert!(found.fixed.is_empty());
    assert_eq!(found.existing.len(), 1);
}

#[test]
fn check_passes_with_only_new_warnings() {
    let found = Delta {
        new: vec![warning("unused_variables", 7, "unused variable")],
        fixed: Vec::new(),
        existing: Vec::new(),
    };

    assert!(verdict(Tier::Check, &found));
}

#[test]
fn check_blocks_on_a_new_error() {
    let found = Delta {
        new: vec![error("E0308", 7, "mismatched types")],
        fixed: Vec::new(),
        existing: Vec::new(),
    };

    assert!(!verdict(Tier::Check, &found));
}

#[test]
fn strict_blocks_on_a_new_warning() {
    let found = Delta {
        new: vec![warning("unused_variables", 7, "unused variable")],
        fixed: Vec::new(),
        existing: Vec::new(),
    };

    assert!(!verdict(Tier::Strict, &found));
}

#[test]
fn model_always_passes_because_the_plan_already_ran_it() {
    let found = Delta {
        new: vec![error("E0308", 7, "mismatched types")],
        fixed: Vec::new(),
        existing: Vec::new(),
    };

    assert!(verdict(Tier::Model, &found));
}
