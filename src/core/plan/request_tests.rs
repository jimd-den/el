// Tests for `validate::request`; sibling of the module they exercise (§4.4).

use super::*;
use crate::core::model::kind::Kind;
use crate::core::plan::fixture;
use crate::core::plan::violation::Code;

fn add(name: &str) -> Change {
    Change::AddDep {
        name: name.to_string(),
    }
}

#[test]
fn a_statement_moved_into_another_body_is_refused() {
    let mut catalog = fixture::catalog();
    let first = fixture::function(&mut catalog, "src/a.rs", "fn first()");
    let second = fixture::function(&mut catalog, "src/a.rs", "fn second()");
    let stmt = catalog.add_stmt(first, "go();").expect("stmt");

    let found = request(&Change::MoveStmt { id: stmt, to: second }, &catalog, &fixture::policy());

    assert_eq!(found[0].code, Code::CrossBodyMove);
    assert_eq!(found[0].id, Some(stmt));
    assert!(found[0].remedy.contains(&first.to_string()), "{}", found[0].remedy);
}

#[test]
fn a_statement_moved_inside_its_own_body_is_allowed() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "go();").expect("stmt");

    assert_eq!(
        request(&Change::MoveStmt { id: stmt, to: run }, &catalog, &fixture::policy()),
        Vec::new()
    );
}

#[test]
fn moving_to_a_target_that_does_not_exist_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let stmt = catalog.add_stmt(run, "go();").expect("stmt");

    let found = request(
        &Change::MoveStmt { id: stmt, to: ElId(404) },
        &catalog,
        &fixture::policy(),
    );

    assert_eq!(found[0].code, Code::UnknownRef);
}

#[test]
fn a_dependency_the_policy_denies_is_refused() {
    let catalog = fixture::catalog();

    let found = request(&add("serde"), &catalog, &fixture::policy());

    assert_eq!(found[0].code, Code::DepDenied);
    assert!(found[0].remedy.contains("serde"), "{}", found[0].remedy);
}

#[test]
fn a_dependency_the_policy_allows_is_accepted() {
    let catalog = fixture::catalog();
    let policy = Policy {
        allowed_deps: vec!["serde".to_string()],
        ..fixture::policy()
    };

    assert_eq!(request(&add("serde"), &catalog, &policy), Vec::new());
}

#[test]
fn a_removal_that_leaves_a_reference_is_refused_and_names_the_referrer() {
    let mut catalog = fixture::catalog();
    let apply = fixture::function(&mut catalog, "src/a.rs", "fn apply(p: &Plan)");
    let caller = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let call = catalog.add_stmt(caller, "apply(p);").expect("stmt");

    let found = request(&Change::Remove { id: apply }, &catalog, &fixture::policy());

    assert_eq!(found[0].code, Code::DanglingRef);
    assert_eq!(found[0].id, Some(call));
    assert!(found[0].remedy.contains("apply"), "{}", found[0].remedy);
}

#[test]
fn a_removal_ignores_references_inside_the_body_it_removes() {
    let mut catalog = fixture::catalog();
    let apply = fixture::function(&mut catalog, "src/a.rs", "fn apply(p: &Plan)");
    catalog.add_stmt(apply, "apply(p);").expect("stmt");

    assert_eq!(
        request(&Change::Remove { id: apply }, &catalog, &fixture::policy()),
        Vec::new()
    );
}

#[test]
fn a_removal_ignores_a_reference_in_a_string() {
    let mut catalog = fixture::catalog();
    let apply = fixture::function(&mut catalog, "src/a.rs", "fn apply(p: &Plan)");
    let caller = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(caller, "log(\"apply\");").expect("stmt");

    assert_eq!(
        request(&Change::Remove { id: apply }, &catalog, &fixture::policy()),
        Vec::new()
    );
}

#[test]
fn a_removal_of_an_unreferenced_element_is_accepted() {
    let mut catalog = fixture::catalog();
    let counter = fixture::item(&mut catalog, Kind::Const, "src/a.rs", "const LIMIT: u8 = 3");

    assert_eq!(
        request(&Change::Remove { id: counter }, &catalog, &fixture::policy()),
        Vec::new()
    );
}
