// FR-18: L0 runs on every mutation, and when it refuses it **writes nothing**.
//
// The honest boundary today is `plan`, not the filesystem. There is no store and
// no binary yet (`src/main.rs` is empty until M5), so "nothing was written" is
// proved the only way that cannot lie: the catalog that goes in is byte-identical
// to the one that comes back, measured on its own text form, and the refusal names
// the element and the measured size. The exit code and the untouched mtime are
// M5's half of this requirement; `docs/PLAN.md` records that they are not here.

use el::core::model::catalog::Catalog;
use el::core::model::body::{Body, BodyStmt};
use el::core::model::change::{Anchor, Change};
use el::core::model::element::{NewElement, Visibility};
use el::core::model::id::ElId;
use el::core::model::kind::Kind;
use el::core::plan::policy::Policy;
use el::core::plan::{plan, Code, Request};

fn ask<'a>(change: &'a Change, policy: &'a Policy) -> Request<'a> {
    Request {
        change,
        policy,
        crate_name: "demo",
    }
}

/// One function with one statement, and a policy that only asks for the rules
/// under test.
fn catalog() -> (Catalog, ElId) {
    let mut catalog = Catalog::new();
    let run = catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/main.rs",
            signature: "fn run()",
            visibility: Visibility::Private,
        })
        .expect("add");
    catalog.add_stmt(run, "go();").expect("stmt");
    (catalog, run)
}

fn policy() -> Policy {
    Policy {
        require_preamble: false,
        ..Policy::default()
    }
}

#[test]
fn a_refused_change_leaves_the_catalog_exactly_as_it_was() {
    let (catalog, run) = catalog();
    let before = catalog.encode();
    let change = Change::AddStmt { owner: run, body: Body { statements: vec![BodyStmt { text: "unsafe { poke(); }".to_string(), children: Vec::new() }] }, anchor: Anchor::End };

    let refused = plan(&catalog, &ask(&change, &policy())).expect_err("forbidden construct");

    assert_eq!(catalog.encode(), before);
    assert_eq!(refused.first().code, Code::Forbidden);
}

#[test]
fn a_refusal_names_the_element_it_is_about() {
    let (catalog, run) = catalog();
    let change = Change::AddStmt { owner: run, body: Body { statements: vec![BodyStmt { text: "unsafe { poke(); }".to_string(), children: Vec::new() }] }, anchor: Anchor::End };

    let refused = plan(&catalog, &ask(&change, &policy())).expect_err("forbidden construct");

    // el-0001 is the function, el-0002 the statement already in it, so the
    // refusal must name el-0003: the statement that carries the construct.
    assert_eq!(refused.first().id, Some(ElId(3)), "the statement, not its function");
    assert!(refused.to_string().contains("el-0003"), "{refused}");
}

#[test]
fn a_refusal_that_oversteps_a_budget_names_the_computed_size() {
    let (catalog, run) = catalog();
    let change = Change::AddStmt { owner: run, body: Body { statements: vec![BodyStmt { text: "more();".to_string(), children: Vec::new() }] }, anchor: Anchor::End };
    let allowed = plan(&catalog, &ask(&change, &policy())).expect("fits any ceiling");
    let lines = allowed
        .projection
        .file("src/main.rs")
        .expect("file")
        .text
        .lines()
        .count();
    let tight = Policy {
        ceiling: lines - 1,
        ..policy()
    };

    let refused = plan(&catalog, &ask(&change, &tight)).expect_err("one line too many");

    assert_eq!(refused.first().code, Code::Ceiling);
    assert_eq!(refused.first().size, Some(lines));
}

#[test]
fn an_allowed_change_still_returns_a_catalog_a_copy_away_from_the_original() {
    let (catalog, run) = catalog();
    let change = Change::AddStmt { owner: run, body: Body { statements: vec![BodyStmt { text: "more();".to_string(), children: Vec::new() }] }, anchor: Anchor::End };

    let outcome = plan(&catalog, &ask(&change, &policy())).expect("allowed");

    assert_eq!(outcome.catalog.statements.len(), 2);
    assert_eq!(catalog.statements.len(), 1, "the original never moved");
}
