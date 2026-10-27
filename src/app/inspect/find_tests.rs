// Tests for `el find`: one query, one fact each. The shared fixture
// carries a `greet` function in `src/greet.rs`; the ordering test builds
// its own catalog so the shared fixture stays untouched.

use super::find;
use super::tests::fixture;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::{NewElement, Visibility};
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;

#[test]
fn finds_an_element_by_its_bare_name() {
    let catalog = fixture();

    let view = find(&catalog, "greet");

    // Both the module and the function are named `greet`; the module sorts
    // first by (module, id), the function second.
    assert_eq!(view.lines.len(), 2);
    assert!(view.lines[0].ends_with("greet::mod/greet"));
    assert!(view.lines[1].ends_with("greet::fn/greet"));
}

#[test]
fn finds_an_element_by_its_module_path() {
    let catalog = fixture();

    let view = find(&catalog, "greet::greet");

    // The module's own path form is `greet::greet` too, so it matches
    // alongside the function.
    assert_eq!(view.lines.len(), 2);
    assert!(view.lines[0].ends_with("greet::mod/greet"));
    assert!(view.lines[1].ends_with("greet::fn/greet"));
}

#[test]
fn finds_an_element_by_its_full_alias() {
    let catalog = fixture();

    let view = find(&catalog, "greet::fn/greet");

    assert_eq!(view.lines.len(), 1);
    assert!(view.lines[0].ends_with("greet::fn/greet"));
}

#[test]
fn returns_no_lines_for_an_unknown_name() {
    let catalog = fixture();

    let view = find(&catalog, "nope");

    assert!(view.lines.is_empty());
}

#[test]
fn lists_matches_in_module_order() {
    let mut catalog = Catalog::new();
    add(&mut catalog, Kind::Module, "src/b.rs", "mod b");
    add(&mut catalog, Kind::Function, "src/b.rs", "fn same()");
    add(&mut catalog, Kind::Module, "src/a.rs", "mod a");
    add(&mut catalog, Kind::Function, "src/a.rs", "fn same()");

    let view = find(&catalog, "same");

    assert_eq!(view.lines.len(), 2);
    assert!(view.lines[0].ends_with("a::fn/same"));
    assert!(view.lines[1].ends_with("b::fn/same"));
}

fn add(catalog: &mut Catalog, kind: Kind, module: &str, signature: &str) -> ElId {
    catalog
        .add_element(&NewElement {
            kind,
            module,
            signature,
            visibility: Visibility::Private,
        })
        .expect("element")
}
