// `show` is the field contract: a named element and an unnamed one, the derived
// doc/source/sequence, and the optional statement tree. The fixture is the same
// one `tree` and `ls` use, so the ids in a failure are recognisable.

use super::tests::{entries, fixture};
use super::*;
use crate::core::render::render;
use crate::core::text::json::Json;

fn field<'a>(payload: &'a Json, key: &str) -> &'a Json {
    payload.get(key).unwrap_or_else(|| panic!("missing {key}"))
}

#[test]
fn show_a_named_element_has_every_documented_field() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(crate::core::model::id::ElId(12)).expect("greet");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: true, source: true, history: true },
    );

    let payload = &view.payload;
    assert_eq!(field(payload, "id").as_str(), Some("el-0012"));
    assert_eq!(field(payload, "kind").as_str(), Some("fn"));
    assert_eq!(field(payload, "alias").as_str(), Some("greet::fn/greet"));
    assert_eq!(field(payload, "module").as_str(), Some("src/greet.rs"));
    assert_eq!(field(payload, "visibility").as_str(), Some("pub"));
    assert_eq!(
        field(payload, "signature").as_str(),
        Some("pub fn greet(name: &str) -> String")
    );
    assert_eq!(field(payload, "doc").as_str(), Some("/// Greets by name."));
    assert_eq!(field(payload, "mode").as_str(), Some("split"));
    assert_eq!(field(payload, "sequence").as_str(), None);
    assert!(matches!(field(payload, "sequence"), Json::Number(_)));
    let statements: Vec<&str> = field(payload, "statements")
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|id| id.as_str())
        .collect();
    assert_eq!(statements, ["el-0013", "el-0014"]);
    assert!(field(payload, "source").as_str().expect("source").contains("format!"));
    assert_eq!(field(payload, "history").as_array().expect("history").len(), 1);
    assert!(view.lines.iter().any(|line| line.starts_with("sequence    12")));
}

#[test]
fn show_a_module_owns_the_entry_that_added_the_module_not_its_function() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(crate::core::model::id::ElId(10)).expect("module");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: false, source: false, history: true },
    );

    assert_eq!(field(&view.payload, "sequence").as_str(), None);
    assert!(matches!(field(&view.payload, "sequence"), Json::Number(n) if n == "2"));
    assert_eq!(field(&view.payload, "history").as_array().expect("history").len(), 1);
}

#[test]
fn show_an_unnamed_element_has_a_null_alias_and_no_doc() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(crate::core::model::id::ElId(7)).expect("impl");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: false, source: true, history: false },
    );

    let payload = &view.payload;
    assert_eq!(field(payload, "alias"), &Json::Null);
    assert_eq!(field(payload, "signature").as_str(), Some("impl Point"));
    assert_eq!(field(payload, "doc"), &Json::Null);
    assert_eq!(field(payload, "sequence"), &Json::Null);
    assert_eq!(field(payload, "statements").as_array().expect("ids").len(), 1);
    assert!(field(payload, "source").as_str().expect("source").contains("impl Point"));
    assert!(!payload.get("statement_tree").is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty())));
}
