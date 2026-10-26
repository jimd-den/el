// Tests for `validate::state`; sibling of the module they exercise (§4.4).
// One test per rule, each asserting the code and the evidence that makes the
// refusal actionable.

use super::*;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;
use crate::core::plan::fixture;
use crate::core::plan::violation::Code;

fn codes(catalog: &Catalog, policy: &Policy) -> Vec<Code> {
    state(catalog, &fixture::projection(catalog), policy).iter().map(|violation| violation.code).collect()
}

#[test]
fn a_clean_catalog_has_no_violations() {
    let mut catalog = fixture::catalog();
    fixture::module(&mut catalog, "src/a.rs");
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(run, "go();").expect("stmt");

    assert_eq!(codes(&catalog, &fixture::policy()), Vec::new());
}

#[test]
fn a_duplicate_alias_names_the_element_that_already_holds_it() {
    let mut catalog = fixture::catalog();
    let first = fixture::function(&mut catalog, "src/a.rs", "fn apply");
    let copy = Element {
        id: ElId(99),
        ..catalog.find(first).expect("added").clone()
    };
    catalog.elements.push(copy);

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::DuplicateAlias);
    assert_eq!(found[0].id, Some(ElId(99)));
    assert!(found[0].remedy.contains(&first.to_string()), "{}", found[0].remedy);
}

#[test]
fn a_statement_with_no_owner_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(run, "go();").expect("stmt");
    catalog.statements[0].owner = ElId(42);

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::UnknownRef);
    assert!(found[0].remedy.contains("el-0042"), "{}", found[0].remedy);
}

#[test]
fn an_element_with_no_module_is_an_unknown_reference() {
    let mut catalog = fixture::catalog();
    let lost = fixture::function(&mut catalog, "src/a.rs", "fn lost");
    catalog.find_mut(lost).expect("added").module.clear();

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::UnknownRef);
    assert_eq!(found[0].id, Some(lost));
}

#[test]
fn a_statement_that_opens_a_block_it_never_closes_is_unbalanced() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(run, "if ready {").expect("stmt");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::Unbalanced);
    assert_eq!(found[0].id, Some(ElId(2)));
}

#[test]
fn a_block_with_children_balances_over_its_subtree() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let block = catalog.add_stmt(run, "if ready {").expect("stmt");
    catalog.add_child_stmt(block, "go();").expect("child");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert!(!found.iter().any(|violation| violation.code == Code::Unbalanced), "{found:?}");
}

#[test]
fn a_nested_block_balances_over_its_subtree() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let outer = catalog.add_stmt(run, "if ready {").expect("stmt");
    let inner = catalog.add_child_stmt(outer, "if set {").expect("child");
    catalog.add_child_stmt(inner, "go();").expect("grandchild");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert!(!found.iter().any(|violation| violation.code == Code::Unbalanced), "{found:?}");
}

#[test]
fn a_child_that_leaves_a_delimiter_open_is_unbalanced() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let block = catalog.add_stmt(run, "if ready {").expect("stmt");
    catalog.add_child_stmt(block, "let v = vec![1, 2;").expect("child");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());
    let unbalanced: Vec<_> =
        found.iter().filter(|violation| violation.code == Code::Unbalanced).collect();

    assert_eq!(unbalanced.len(), 1, "{found:?}");
    assert_eq!(unbalanced[0].id, Some(block));
}

#[test]
fn a_head_with_an_unclosed_parenthesis_is_unbalanced() {
    let mut catalog = fixture::catalog();
    fixture::function(&mut catalog, "src/a.rs", "fn run(p: &Plan");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::Unbalanced);
}

#[test]
fn a_reserved_keyword_as_a_name_is_a_bad_identifier() {
    let mut catalog = fixture::catalog();
    fixture::function(&mut catalog, "src/a.rs", "fn match");

    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::BadIdent);
    assert!(found[0].remedy.contains("\"match\""), "{}", found[0].remedy);
}

#[test]
fn a_forbidden_construct_is_found_where_it_is_code_not_in_a_string() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let comment = catalog.add_stmt(run, "log(\"unsafe\");").expect("stmt");

    assert_eq!(codes(&catalog, &fixture::policy()), Vec::new(), "a string is not a construct");

    catalog.add_stmt(run, "unsafe { poke(); }").expect("stmt");
    let found = state(&catalog, &fixture::projection(&catalog), &fixture::policy());

    assert_eq!(found[0].code, Code::Forbidden);
    assert_ne!(found[0].id, Some(comment));
    assert!(found[0].remedy.contains("unsafe"), "{}", found[0].remedy);
}
