// Anchor, reorder and comment-attachment tests, separate from the append/edit
// basics so each file stays under the ceiling. The duplicate `body` helper is the
// price of not making a test helper public across modules.

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

#[test]
fn an_insert_after_and_before_an_anchor_keeps_sibling_order() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let first = catalog.add_stmt(run, "a();").expect("stmt");
    let last = catalog.add_stmt(run, "b();").expect("stmt");

    apply(
        &mut catalog,
        &Change::AddStmt {
            owner: run,
            body: body("middle();"),
            anchor: Anchor::After(first),
        },
    )
    .expect("after");
    let middle = catalog.children_of(run)[1].id;
    apply(
        &mut catalog,
        &Change::AddStmt {
            owner: run,
            body: body("before();"),
            anchor: Anchor::Before(first),
        },
    )
    .expect("before");

    let before = catalog.children_of(run)[0].id;
    let order: Vec<ElId> = catalog.children_of(run).iter().map(|stmt| stmt.id).collect();
    assert_eq!(order, vec![before, first, middle, last]);
    assert_ne!(before, first);
}

#[test]
fn an_insert_at_a_position_reindexes_the_siblings() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let first = catalog.add_stmt(run, "a();").expect("stmt");
    let last = catalog.add_stmt(run, "b();").expect("stmt");

    apply(
        &mut catalog,
        &Change::AddStmt {
            owner: run,
            body: body("middle();"),
            anchor: Anchor::At(1),
        },
    )
    .expect("at");

    let order: Vec<ElId> = catalog.children_of(run).iter().map(|stmt| stmt.id).collect();
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], first);
    assert_eq!(order[2], last);
    let indices: Vec<u32> = catalog.children_of(run).iter().map(|stmt| stmt.index).collect();
    assert_eq!(indices, vec![0, 1, 2]);
}

#[test]
fn an_insert_can_bring_a_block_and_its_children() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let block = BodyStmt {
        text: "if ready {".to_string(),
        children: vec![BodyStmt {
            text: "go();".to_string(),
            children: Vec::new(),
        }],
    };

    apply(
        &mut catalog,
        &Change::AddStmt {
            owner: run,
            body: Body { statements: vec![block] },
            anchor: Anchor::End,
        },
    )
    .expect("inserted");

    let outer = catalog.children_of(run)[0];
    assert_eq!(catalog.children_of(outer.id).len(), 1);
}

#[test]
fn reordering_after_a_sibling_keeps_ids_and_sets_indices() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let first = catalog.add_stmt(run, "a();").expect("stmt");
    let second = catalog.add_stmt(run, "b();").expect("stmt");
    let third = catalog.add_stmt(run, "c();").expect("stmt");

    apply(
        &mut catalog,
        &Change::ReorderStmt {
            id: first,
            after: third,
        },
    )
    .expect("reordered");

    let order: Vec<ElId> = catalog.children_of(run).iter().map(|stmt| stmt.id).collect();
    assert_eq!(order, vec![second, third, first]);
    assert_eq!(catalog.find_stmt(first).expect("id").index, 2);
}

#[test]
fn an_edit_replaces_the_children_it_owned() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let block = catalog.add_stmt(run, "if ready {").expect("stmt");
    catalog.add_child_stmt(block, "old();").expect("child");

    apply(
        &mut catalog,
        &Change::EditStmt {
            id: block,
            body: Body {
                statements: vec![BodyStmt {
                    text: "while ready {".to_string(),
                    children: vec![BodyStmt {
                        text: "new();".to_string(),
                        children: Vec::new(),
                    }],
                }],
            },
        },
    )
    .expect("edited");

    assert_eq!(catalog.find_stmt(block).expect("id").text, "while ready {");
    let children = catalog.children_of(block);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].text, "new();");
}

#[test]
fn a_comment_attached_to_a_statement_moves_with_it() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let first = catalog.add_stmt(run, "// note\nlet a = 1;").expect("stmt");
    let second = catalog.add_stmt(run, "let b = 2;").expect("stmt");

    apply(
        &mut catalog,
        &Change::ReorderStmt {
            id: first,
            after: second,
        },
    )
    .expect("reordered");

    assert_eq!(
        catalog.find_stmt(first).expect("first").text,
        "// note\nlet a = 1;"
    );
    let order: Vec<ElId> = catalog.children_of(run).iter().map(|stmt| stmt.id).collect();
    assert_eq!(order, vec![second, first]);
}
