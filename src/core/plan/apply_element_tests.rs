// Tests for the element half of `apply`; sibling of the module they exercise.

use super::*;
use crate::core::model::body::Body;
use crate::core::model::element::Visibility;
use crate::core::model::kind::Kind;
use crate::core::plan::fixture;

fn add(signature: &str) -> Change {
    Change::AddElement {
        kind: Kind::Function,
        module: "src/a.rs".to_string(),
        signature: signature.to_string(),
        visibility: Visibility::Private,
        body: Body::default(),
    }
}

#[test]
fn an_add_creates_the_element_it_describes() {
    let mut catalog = fixture::catalog();

    apply(&mut catalog, &add("fn apply(p: &Plan)")).expect("applied");

    let element = &catalog.elements[0];
    assert_eq!(element.name.as_deref(), Some("apply"));
    assert_eq!(element.head, "(p: &Plan)");
    assert_eq!(element.module, "src/a.rs");
}

#[test]
fn an_add_with_a_bad_signature_is_refused_and_adds_nothing() {
    let mut catalog = fixture::catalog();

    let refused = apply(&mut catalog, &add("struct Point"));

    assert_eq!(refused.expect_err("refused").code, Code::BadIdent);
    assert!(catalog.elements.is_empty());
}

#[test]
fn an_add_that_would_duplicate_an_alias_names_the_holder() {
    let mut catalog = fixture::catalog();
    let first = fixture::function(&mut catalog, "src/a.rs", "fn apply");

    let refused = apply(&mut catalog, &add("fn apply(p: &Plan)"));

    assert_eq!(refused.expect_err("refused").id, Some(first));
    assert_eq!(catalog.elements.len(), 1);
}

#[test]
fn an_edit_replaces_the_head_and_keeps_the_id() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");

    apply(
        &mut catalog,
        &Change::EditElement {
            id: run,
            head: Some("(n: usize) -> usize".to_string()),
            visibility: None,
            body: None,
        },
    )
    .expect("applied");

    let element = catalog.find(run).expect("still there");
    assert_eq!(element.id, run);
    assert_eq!(element.head, "(n: usize) -> usize");
    assert_eq!(element.name.as_deref(), Some("run"));
}

#[test]
fn an_edit_of_an_unknown_id_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();

    let refused = apply(
        &mut catalog,
        &Change::EditElement {
            id: crate::core::model::id::ElId(7),
            head: Some("()".to_string()),
            visibility: None,
            body: None,
        },
    );

    assert_eq!(refused.expect_err("refused").code, Code::UnknownRef);
}

#[test]
fn a_move_changes_the_module_and_the_alias_and_keeps_the_id() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");

    apply(
        &mut catalog,
        &Change::MoveElement {
            id: run,
            module: "src/core/b.rs".to_string(),
        },
    )
    .expect("applied");

    let element = catalog.find(run).expect("still there");
    assert_eq!(element.module, "src/core/b.rs");
    assert_eq!(
        element.alias.as_ref().map(|alias| alias.as_str()),
        Some("core::b::fn/run")
    );
}

#[test]
fn a_rename_keeps_the_head_and_the_id() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run(p: &Plan)");

    apply(
        &mut catalog,
        &Change::Rename {
            id: run,
            name: "walk".to_string(),
        },
    )
    .expect("applied");

    let element = catalog.find(run).expect("still there");
    assert_eq!(element.name.as_deref(), Some("walk"));
    assert_eq!(element.head, "(p: &Plan)");
}

#[test]
fn a_remove_takes_the_body_with_it() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "go();").expect("stmt");

    apply(&mut catalog, &Change::Remove { id: run }).expect("applied");

    assert!(catalog.elements.is_empty());
    assert!(catalog.find_stmt(stmt).is_none());
}

#[test]
fn a_remove_of_an_unknown_id_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();

    let refused = apply(
        &mut catalog,
        &Change::Remove {
            id: crate::core::model::id::ElId(9),
        },
    );

    assert_eq!(refused.expect_err("refused").code, Code::UnknownRef);
}

#[test]
fn a_statement_change_is_not_this_modules_business() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");

    apply(
        &mut catalog,
        &Change::AddStmt {
            owner: run,
            text: "go();".to_string(),
        },
    )
    .expect("left for apply_stmt");

    assert!(catalog.statements.is_empty());
}

#[test]
fn a_remove_of_a_statement_takes_its_children_with_it() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let outer = catalog.add_stmt(run, "loop {").expect("stmt");
    let inner = catalog.add_child_stmt(outer, "break;").expect("stmt");

    apply(&mut catalog, &Change::Remove { id: outer }).expect("applied");

    assert!(catalog.find_stmt(outer).is_none());
    assert!(catalog.find_stmt(inner).is_none());
    assert_eq!(catalog.find(run).expect("owner survives").id, run);
}
