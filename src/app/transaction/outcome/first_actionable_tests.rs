// The choice is a rule, not a scan: a touched element beats an untouched one, an
// exact attribution beats an inexact one, and the compiler's order is the final
// tie-break. Each case is one line of that rule.

use super::first_actionable;
use crate::core::gate::diagnostics::{Diagnostic, Level};
use crate::core::model::id::ElId;

fn diag(id: Option<u32>, exact: bool) -> Diagnostic {
    Diagnostic {
        code: "E0308".to_string(),
        level: Level::Error,
        el: id.map(ElId),
        owner: None,
        path: "src/main.rs".to_string(),
        line: 1,
        col: 1,
        message: "mismatched types".to_string(),
        exact,
    }
}

#[test]
fn a_touched_element_beats_an_earlier_untouched_one() {
    let list = [diag(Some(1), true), diag(Some(2), true)];

    assert_eq!(first_actionable(&list, &[ElId(2)]), Some(1));
}

#[test]
fn an_exact_attribution_beats_an_inexact_one() {
    let list = [diag(None, false), diag(Some(4), true)];

    assert_eq!(first_actionable(&list, &[]), Some(1));
}

#[test]
fn the_first_diagnostic_is_the_last_tie_break() {
    let list = [diag(None, false), diag(None, false)];

    assert_eq!(first_actionable(&list, &[]), Some(0));
}

#[test]
fn no_diagnostics_means_no_mark() {
    assert_eq!(first_actionable(&[], &[]), None);
}
