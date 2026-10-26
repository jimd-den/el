// Cargo JSON parsing against real compiler output, checked in as fixtures.
// The fixtures are captured `cargo check` / `cargo clippy` runs, not invented
// shapes, so a cargo upgrade that changes the frames breaks a test here first.

use super::parse_cargo_json;
use super::super::diagnostics::Level;
use crate::core::model::id::ElId;

fn never_attribute(_path: &str, _line: u32) -> Option<(ElId, Option<ElId>)> {
    None
}

fn attribute_main_line_3(path: &str, line: u32) -> Option<(ElId, Option<ElId>)> {
    if path == "src/main.rs" && line == 3 {
        Some((ElId(7), Some(ElId(2))))
    } else {
        None
    }
}

#[test]
fn an_e0601_frame_parses_and_its_failure_note_is_ignored() {
    let text = include_str!("../../../tests/fixtures/cargo-json/e0601.jsonl");

    let found = parse_cargo_json(text, &never_attribute);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E0601");
    assert_eq!(found[0].path, "src/main.rs");
    assert_eq!(found[0].line, 3);
    assert!(!found[0].exact);
}

#[test]
fn duplicate_frames_are_kept_raw_for_the_delta_to_collapse() {
    let text = include_str!("../../../tests/fixtures/cargo-json/warning_and_e0308.jsonl");

    let found = parse_cargo_json(text, &never_attribute);

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|diag| diag.code == "E0308"));
}

#[test]
fn a_warning_frame_parses_with_its_warning_level() {
    let text = include_str!("../../../tests/fixtures/cargo-json/warning.jsonl");

    let found = parse_cargo_json(text, &never_attribute);

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|diag| diag.code == "unused_variables"));
    assert!(found.iter().all(|diag| diag.level == Level::Warning));
}

#[test]
fn a_clippy_code_arrives_already_namespaced_and_is_kept_as_is() {
    let text = include_str!("../../../tests/fixtures/cargo-json/clippy.jsonl");

    let found = parse_cargo_json(text, &never_attribute);

    assert_eq!(found.len(), 2);
    assert!(found
        .iter()
        .all(|diag| diag.code == "clippy::needless_return"));
}

#[test]
fn an_attributed_span_names_the_statement_and_its_owner() {
    // The E0601 fixture used to carry this test; ADR-0027 deliberately
    // un-attributes E0601, so the same intent rides an E0425 frame instead.
    let text = "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"code\":{\"code\":\"E0425\"},\"message\":\"cannot find value `x` in this scope\",\"spans\":[{\"file_name\":\"src/main.rs\",\"line_start\":3,\"column_start\":5,\"is_primary\":true}]}}\n";

    let found = parse_cargo_json(text, &attribute_main_line_3);

    assert_eq!(found.len(), 1);
    assert!(found[0].exact);
    assert_eq!(found[0].el, Some(ElId(7)));
    assert_eq!(found[0].owner, Some(ElId(2)));
}

#[test]
fn an_e0601_is_never_attributed_because_it_names_the_crate() {
    let text = include_str!("../../../tests/fixtures/cargo-json/e0601.jsonl");

    let found = parse_cargo_json(text, &attribute_main_line_3);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "E0601");
    assert_eq!(found[0].el, None);
    assert!(!found[0].exact);
}

#[test]
fn non_compiler_message_frames_are_ignored() {
    let text = "{\"reason\":\"compiler-artifact\",\"package_id\":\"x\"}\n{\"reason\":\"build-finished\",\"success\":false}\n";

    let found = parse_cargo_json(text, &never_attribute);

    assert!(found.is_empty());
}

#[test]
fn a_malformed_line_is_skipped_not_fatal() {
    let text = "this is not json\n{\"reason\":\"build-finished\",\"success\":true}\n";

    let found = parse_cargo_json(text, &never_attribute);

    assert!(found.is_empty());
}
