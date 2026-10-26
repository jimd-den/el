// Tests for the batch input shape: one item, one fact each. Parsing is
// pure, so these run without a crate; the transaction half is covered by
// the acceptance tests.

use super::parse_batch;
use crate::cli::request::Source;
use crate::core::model::element::Visibility;
use crate::core::model::kind::Kind;

fn single(kind: &str) -> String {
    format!(
        "[{{\"kind\":\"{kind}\",\"name\":\"escape\",\"in\":\"json\",\"vis\":\"pub\",\"sig\":\"fn escape(s: &str) -> String\",\"body\":\"s.to_string()\"}}]"
    )
}

#[test]
fn parses_a_valid_item_into_an_add() {
    let adds = parse_batch(&single("fn")).expect("parse");

    assert_eq!(adds.len(), 1);
    assert_eq!(adds[0].kind, Kind::Function);
    assert_eq!(adds[0].name, "escape");
    assert_eq!(adds[0].module.as_deref(), Some("json"));
    assert_eq!(adds[0].visibility, Some(Visibility::Pub));
    assert_eq!(adds[0].signature.as_deref(), Some("fn escape(s: &str) -> String"));
    assert_eq!(adds[0].body, Some(Source::Literal("s.to_string()".to_string())));
}

#[test]
fn a_module_item_carries_its_body_as_a_preamble() {
    let adds = parse_batch("[{\"kind\":\"mod\",\"name\":\"json\",\"body\":\"// json\"}]").expect("parse");

    assert_eq!(adds[0].preamble, Some(Source::Literal("// json".to_string())));
    assert_eq!(adds[0].body, None);
}

#[test]
fn a_missing_kind_names_its_item() {
    let error = parse_batch("[{\"name\":\"escape\"}]").expect_err("must refuse");

    assert!(error.message.contains("item 0"), "message: {}", error.message);
}

#[test]
fn an_unknown_kind_names_its_item() {
    let error = parse_batch(&single("widget")).expect_err("must refuse");

    assert!(error.message.contains("item 0"), "message: {}", error.message);
    assert!(error.message.contains("widget"), "message: {}", error.message);
}

#[test]
fn a_statement_item_is_refused() {
    let error = parse_batch(&single("stmt")).expect_err("must refuse");

    assert!(error.message.contains("stmt"), "message: {}", error.message);
}

#[test]
fn an_empty_array_is_refused() {
    let error = parse_batch("[]").expect_err("must refuse");

    assert!(error.message.contains("at least one"), "message: {}", error.message);
}

#[test]
fn a_non_array_is_refused() {
    let error = parse_batch("{\"kind\":\"fn\"}").expect_err("must refuse");

    assert!(error.message.contains("array"), "message: {}", error.message);
}

#[test]
fn a_bad_visibility_names_its_item() {
    let error = parse_batch(
        "[{\"kind\":\"fn\",\"name\":\"escape\",\"vis\":\"banana\",\"sig\":\"fn escape()\"}]",
    )
    .expect_err("must refuse");

    assert!(error.message.contains("item 0"), "message: {}", error.message);
    assert!(error.message.contains("banana"), "message: {}", error.message);
}
