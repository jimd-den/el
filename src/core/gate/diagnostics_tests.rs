// Fingerprint and normalization: the identity a diagnostic keeps across edits.

use super::{normalize, Diagnostic, Level};
use crate::core::model::id::ElId;

fn exact(code: &str, id: u32, line: u32, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        level: Level::Error,
        el: Some(ElId(id)),
        owner: Some(ElId(1)),
        path: "src/main.rs".to_string(),
        line,
        col: 1,
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
        path: "src/main.rs".to_string(),
        line,
        col: 1,
        message: message.to_string(),
        exact: false,
    }
}

#[test]
fn an_exact_fingerprint_ignores_the_line_it_came_from() {
    let above = exact("E0308", 7, 3, "mismatched types");
    let below = exact("E0308", 7, 30, "mismatched types");

    assert_eq!(above.fingerprint(), below.fingerprint());
}

#[test]
fn an_exact_fingerprint_changes_with_the_element_or_the_message() {
    let one = exact("E0308", 7, 3, "mismatched types");
    let other_element = exact("E0308", 9, 3, "mismatched types");
    let other_message = exact("E0308", 7, 3, "unresolved name");

    assert_ne!(one.fingerprint(), other_element.fingerprint());
    assert_ne!(one.fingerprint(), other_message.fingerprint());
}

#[test]
fn a_fallback_fingerprint_ignores_the_line_it_came_from() {
    let above = fallback("E0601", 5, "`main` function not found in crate `hello`");
    let below = fallback("E0601", 9, "`main` function not found in crate `hello`");

    assert_eq!(above.fingerprint(), below.fingerprint());
}

#[test]
fn a_fallback_fingerprint_changes_with_the_code_path_or_message() {
    let one = fallback("E0601", 5, "`main` function not found in crate `hello`");
    let other_code = fallback("E0308", 5, "`main` function not found in crate `hello`");
    let other_message = fallback("E0601", 5, "unresolved name");

    assert_ne!(one.fingerprint(), other_code.fingerprint());
    assert_ne!(one.fingerprint(), other_message.fingerprint());
}

#[test]
fn normalize_collapses_whitespace_and_erases_absolute_paths() {
    let raw = "couldn't read\n  /tmp/work/demo/src/main.rs   : No such file";

    assert_eq!(
        normalize(raw),
        "couldn't read <path> : No such file"
    );
}

#[test]
fn normalize_keeps_relative_paths_and_type_names() {
    let raw = "mismatched types: expected `Vec<T>`, found `Vec<i32>` in src/main.rs";

    assert_eq!(normalize(raw), raw);
}
