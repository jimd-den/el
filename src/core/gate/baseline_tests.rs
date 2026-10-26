// Baseline file round-trips: tier-tagged lines, malformed lines skipped.

use super::{decode, encode};
use crate::core::gate::diagnostics::{Diagnostic, Level};
use crate::core::model::id::ElId;

fn error(code: &str, id: u32, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        level: Level::Error,
        el: Some(ElId(id)),
        owner: Some(ElId(2)),
        path: "src/main.rs".to_string(),
        line: 3,
        col: 9,
        message: message.to_string(),
        exact: true,
    }
}

fn fallback(code: &str, line: u32, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        level: Level::Error,
        el: None,
        owner: None,
        path: "src/greet.rs".to_string(),
        line,
        col: 1,
        message: message.to_string(),
        exact: false,
    }
}

#[test]
fn encode_decode_round_trips_exact_and_fallback_diagnostics() {
    let diags = vec![
        error("E0308", 7, "mismatched types"),
        fallback("E0601", 3, "main function not found"),
    ];

    let back = decode(&encode("check", &diags), "check");

    assert_eq!(back, diags);
}

#[test]
fn decode_ignores_lines_tagged_with_another_tier() {
    let check = encode("check", &[error("E0308", 7, "mismatched types")]);
    let strict = encode("strict", &[error("E0425", 9, "unresolved name")]);
    let both = format!("{check}{strict}");

    let back = decode(&both, "check");

    assert_eq!(back.len(), 1);
    assert_eq!(back[0].code, "E0308");
}

#[test]
fn decode_skips_a_malformed_line_and_keeps_the_good_one() {
    let good = encode("check", &[error("E0308", 7, "mismatched types")]);
    let text = format!("garbage line\n{good}el1\tcheck\tbroken\n");

    let back = decode(&text, "check");

    assert_eq!(back.len(), 1);
    assert_eq!(back[0].code, "E0308");
}

#[test]
fn decode_of_empty_text_is_an_empty_set() {
    assert!(decode("", "check").is_empty());
}

#[test]
fn encode_sanitizes_tabs_and_newlines_inside_messages() {
    let diag = error("E0308", 7, "first\tsecond\nthird");

    let text = encode("check", &[diag]);
    let back = decode(&text, "check");

    assert_eq!(text.lines().count(), 1);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].message, "first second third");
}
