// Tests for the statement half of `apply`; sibling of the module they exercise.

use super::*;
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::change::Anchor;
use crate::core::plan::fixture;

fn body(text: &str) -> Body {
    Body {
        statements: vec![BodyStmt {
            text: text.to_string(),
            children: Vec::new(),
        }],
    }
}

fn add(owner: ElId, text: &str) -> Change {
    Change::AddStmt {
        owner,
        body: body(text),
        anchor: Anchor::End,
    }
}

#[test]
fn an_add_appends_in_order() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");

    apply(&mut catalog, &add(run, "a();")).expect("applied");
    apply(&mut catalog, &add(run, "b();")).expect("applied");

    let texts: Vec<&str> = catalog
        .children_of(run)
        .iter()
        .map(|stmt| stmt.text.as_str())
        .collect();
    assert_eq!(texts, vec!["a();", "b();"]);
}

#[test]
fn an_add_to_an_unknown_owner_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();

    let refused = apply(&mut catalog, &add(ElId(8), "a();"));

    assert_eq!(refused.expect_err("refused").code, Code::UnknownRef);
    assert!(catalog.statements.is_empty());
}

#[test]
fn an_edit_keeps_the_id_and_the_index() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "a();").expect("stmt");

    apply(
        &mut catalog,
        &Change::EditStmt {
            id: stmt,
            body: body("b();"),
        },
    )
    .expect("applied");

    let edited = catalog.find_stmt(stmt).expect("still there");
    assert_eq!(edited.text, "b();");
    assert_eq!(edited.index, 0);
}

#[test]
fn an_edit_of_an_unknown_statement_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();

    let refused = apply(
        &mut catalog,
        &Change::EditStmt {
            id: ElId(11),
            body: body("b();"),
        },
    );

    assert_eq!(refused.expect_err("refused").code, Code::UnknownRef);
}

#[test]
fn a_move_inside_its_own_body_reorders_it() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let first = catalog.add_stmt(run, "a();").expect("stmt");
    catalog.add_stmt(run, "b();").expect("stmt");

    apply(&mut catalog, &Change::MoveStmt { id: first, to: run }).expect("applied");

    let order: Vec<ElId> = catalog.children_of(run).iter().map(|stmt| stmt.id).collect();
    assert_eq!(order.last(), Some(&first));
}

#[test]
fn a_move_into_another_body_is_refused_by_apply_too() {
    let mut catalog = fixture::catalog();
    let first = fixture::function(&mut catalog, "src/a.rs", "fn first()");
    let second = fixture::function(&mut catalog, "src/a.rs", "fn second()");
    let stmt = catalog.add_stmt(first, "go();").expect("stmt");

    let refused = apply(&mut catalog, &Change::MoveStmt { id: stmt, to: second });

    assert_eq!(refused.expect_err("refused").code, Code::CrossBodyMove);
    assert_eq!(catalog.find_stmt(stmt).expect("unmoved").owner, first);
}

#[test]
fn a_move_to_a_target_that_does_not_exist_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "go();").expect("stmt");

    let refused = apply(&mut catalog, &Change::MoveStmt { id: stmt, to: ElId(404) });

    assert_eq!(refused.expect_err("refused").code, Code::UnknownRef);
}

#[test]
fn a_removal_is_not_this_modules_business() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "go();").expect("stmt");

    apply(&mut catalog, &Change::Remove { id: stmt }).expect("left for apply_element");

    assert!(catalog.find_stmt(stmt).is_some());
}
