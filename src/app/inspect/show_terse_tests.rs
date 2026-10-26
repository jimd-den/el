// `show --terse`: the one-line header and the statement listing that carries
// full bodies. The fixture is the same one the field-contract tests use, so
// the ids in a failure are recognisable.

use super::tests::{entries, fixture};
use super::*;
use crate::core::render::render;

#[test]
fn show_a_function_terse_is_one_line() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(crate::core::model::id::ElId(12)).expect("greet");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: false, source: false, history: false, terse: true },
    );

    assert_eq!(
        view.lines,
        ["el-0012 fn greet::fn/greet src/greet.rs pub fn greet(name: &str) -> String"]
    );
}

#[test]
fn show_a_statement_terse_is_one_line_with_owner_parent_and_text() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let stmt = catalog.find_stmt(crate::core::model::id::ElId(14)).expect("statement");

    let view = statement(
        &catalog,
        &projection,
        &entries,
        stmt,
        &Options { statements: false, source: false, history: false, terse: true },
    );

    assert_eq!(
        view.lines,
        ["el-0014 stmt owner el-0012 parent - format!(\"hello, {name}\")"]
    );
}

#[test]
fn show_statements_lists_a_multi_line_statement_in_full() {
    let mut catalog = fixture();
    let greet = crate::core::model::id::ElId(12);
    catalog.add_stmt(greet, "vec![\n    1,\n]").expect("multi-line");
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(greet).expect("greet");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: true, source: false, history: false, terse: false },
    );

    let text = view.lines.join("\n");
    assert!(text.contains("vec!["), "{text}");
    assert!(text.contains("    1,"), "{text}");
    assert!(text.contains("]"), "{text}");
}

#[test]
fn show_a_function_terse_with_statements_lists_bodies_under_the_header() {
    let catalog = fixture();
    let projection = render(&catalog, "demo");
    let entries = entries();
    let element = catalog.find(crate::core::model::id::ElId(12)).expect("greet");

    let view = show(
        &catalog,
        &projection,
        &entries,
        element,
        &Options { statements: true, source: false, history: false, terse: true },
    );

    assert_eq!(
        view.lines[0],
        "el-0012 fn greet::fn/greet src/greet.rs pub fn greet(name: &str) -> String"
    );
    assert!(!view.lines.iter().any(|line| line == "statements:"));
    assert!(view.lines.iter().any(|line| line.contains("el-0013")), "{:?}", view.lines);
    assert!(view.lines.iter().any(|line| line.contains("el-0014")), "{:?}", view.lines);
    assert!(
        view.lines.iter().any(|line| line.contains("format!(\"hello, {name}\")")),
        "{:?}",
        view.lines
    );
}

