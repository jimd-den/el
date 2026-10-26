// Tests for `plan` itself: the boundary where a change becomes an outcome or a
// refusal, and where purity is proved rather than asserted.

use super::*;
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::change::Anchor;
use crate::core::model::element::{BodyMode, Visibility};
use crate::core::model::kind::Kind;
use crate::core::plan::fixture;

fn one(text: &str) -> Body {
    Body {
        statements: vec![BodyStmt {
            text: text.to_string(),
            children: Vec::new(),
        }],
    }
}

fn ask<'a>(change: &'a Change, policy: &'a policy::Policy) -> Request<'a> {
    Request {
        change,
        policy,
        crate_name: "demo",
    }
}

#[test]
fn a_change_that_passes_returns_a_new_catalog_and_its_projection() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let change = Change::AddStmt { owner: run, body: one("go();"), anchor: Anchor::End };

    let outcome = plan(&catalog, &ask(&change, &fixture::policy())).expect("allowed");

    assert_eq!(outcome.catalog.statements.len(), 1);
    assert!(outcome.projection.file("src/a.rs").is_some());
    let stmt = outcome
        .projection
        .spans
        .iter()
        .find(|span| span.element == ElId(2))
        .expect("the statement has a span");
    assert_eq!(outcome.projection.element_at("src/a.rs", stmt.first), Some(ElId(2)));
    assert!(outcome.rewrites.is_empty());
}

#[test]
fn the_catalog_you_passed_in_is_not_touched() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let before = catalog.encode();
    let change = Change::AddStmt { owner: run, body: one("go();"), anchor: Anchor::End };

    let _outcome = plan(&catalog, &ask(&change, &fixture::policy())).expect("allowed");

    assert_eq!(catalog.encode(), before, "plan must work on a copy");
}

#[test]
fn a_ceiling_violation_is_refused_and_names_the_size_and_a_seam() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(run, "a();").expect("stmt");
    catalog.add_stmt(run, "b();").expect("stmt");
    let policy = policy::Policy {
        ceiling: 4,
        ..fixture::policy()
    };
    let change = Change::AddStmt { owner: run, body: one("c();"), anchor: Anchor::End };

    let refused = plan(&catalog, &ask(&change, &policy)).expect_err("over the ceiling");

    assert_eq!(refused.first().code, Code::Ceiling);
    assert!(refused.first().size.is_some_and(|size| size > 4));
    assert!(refused.first().remedy.contains("extract statement"), "{refused}");
    assert_eq!(catalog.statements.len(), 2, "nothing was added");
}

#[test]
fn a_refusal_reports_the_rules_in_cheapness_order() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = policy::Policy {
        ceiling: 3,
        ..fixture::policy()
    };
    let change = Change::AddStmt { owner: run, body: one("if ready {"), anchor: Anchor::End };

    let refused = plan(&catalog, &ask(&change, &policy)).expect_err("two rules");

    assert_eq!(refused.codes(), vec![Code::Unbalanced, Code::Ceiling]);
}

#[test]
fn a_rename_reports_the_references_it_rewrote_and_leaves_the_rest() {
    let mut catalog = fixture::catalog();
    let apply = fixture::function(&mut catalog, "src/a.rs", "fn apply(p: &Plan)");
    let caller = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog
        .add_stmt(caller, "apply(p); // apply\nlog(\"apply\");")
        .expect("stmt");
    let change = Change::Rename {
        id: apply,
        name: "dispatch".to_string(),
    };

    let outcome = plan(&catalog, &ask(&change, &fixture::policy())).expect("allowed");

    assert_eq!(outcome.rewrites.len(), 1);
    assert_eq!(outcome.rewrites[0].after, "dispatch(p); // apply\nlog(\"apply\");");
    assert_eq!(
        outcome.catalog.find(apply).and_then(|element| element.name.clone()),
        Some("dispatch".to_string())
    );
    assert_eq!(catalog.find(apply).and_then(|e| e.name.clone()), Some("apply".to_string()));
}

#[test]
fn a_removal_that_leaves_a_reference_is_refused_before_it_is_applied() {
    let mut catalog = fixture::catalog();
    let apply = fixture::function(&mut catalog, "src/a.rs", "fn apply(p: &Plan)");
    let caller = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(caller, "apply(p);").expect("stmt");
    let change = Change::Remove { id: apply };

    let refused = plan(&catalog, &ask(&change, &fixture::policy())).expect_err("dangling");

    assert_eq!(refused.first().code, Code::DanglingRef);
    assert_eq!(catalog.elements.len(), 2, "nothing was removed");
}

#[test]
fn an_allowed_dependency_is_refused_rather_than_silently_dropped() {
    let catalog = fixture::catalog();
    let policy = policy::Policy {
        allowed_deps: vec!["serde".to_string()],
        ..fixture::policy()
    };
    let change = Change::AddDep {
        name: "serde".to_string(),
    };

    let refused = plan(&catalog, &ask(&change, &policy)).expect_err("cannot record it");

    assert_eq!(refused.first().code, Code::DepDenied);
    assert!(refused.first().remedy.contains("cannot record"), "{refused}");
}

#[test]
fn a_denied_dependency_is_refused_with_the_policy_reason() {
    let catalog = fixture::catalog();
    let change = Change::AddDep {
        name: "serde".to_string(),
    };

    let refused = plan(&catalog, &ask(&change, &fixture::policy())).expect_err("denied");

    assert!(refused.first().remedy.contains("list it under `deps`"), "{refused}");
}

#[test]
fn a_new_item_arrives_with_its_kind_its_module_and_its_visibility() {
    let catalog = fixture::catalog();
    let change = Change::AddElement {
        kind: Kind::Struct,
        module: "src/a.rs".to_string(),
        signature: "struct Point { x: u8 }".to_string(),
        visibility: Visibility::Pub,
        body: Body::default(),
        mode: BodyMode::Split,
    };

    let outcome = plan(&catalog, &ask(&change, &fixture::policy())).expect("allowed");

    let element = &outcome.catalog.elements[0];
    assert_eq!(element.kind, Kind::Struct);
    assert_eq!(element.visibility, Visibility::Pub);
    assert!(outcome.projection.file("src/a.rs").expect("file").text.contains("pub struct Point"));
    assert_eq!(catalog.elements.len(), 0);
}
