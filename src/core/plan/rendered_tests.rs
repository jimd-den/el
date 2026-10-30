// Tests for `rendered`; sibling of the module they exercise (§4.4).

use super::*;
use crate::core::plan::fixture;
use crate::core::plan::violation::Code;

fn rendered_violations(catalog: &Catalog, policy: &Policy) -> Vec<Violation> {
    let mut found = Vec::new();
    checks(catalog, &fixture::projection(catalog), policy, &mut found);
    found
}

#[test]
fn a_file_over_the_ceiling_names_its_size_and_a_statement_to_extract() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    for text in ["a();", "b();", "c();"] {
        catalog.add_stmt(run, text).expect("stmt");
    }
    let policy = Policy {
        ceiling: 3,
        ..fixture::policy()
    };
    let projection = fixture::projection(&catalog);
    let lines = projection.file("src/a.rs").expect("file").text.lines().count();

    let found = rendered_violations(&catalog, &policy);

    assert_eq!(found[0].code, Code::Ceiling);
    assert_eq!(found[0].size, Some(lines));
    assert!(found[0].remedy.contains("extract statement el-"), "{}", found[0].remedy);
}

#[test]
fn a_file_over_the_ceiling_says_so_when_nothing_is_extractable() {
    let mut catalog = fixture::catalog();
    fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = Policy {
        ceiling: 1,
        ..fixture::policy()
    };

    let found = rendered_violations(&catalog, &policy);

    assert_eq!(found[0].code, Code::Ceiling);
    assert!(found[0].remedy.contains("no single statement"), "{}", found[0].remedy);
}

#[test]
fn an_element_over_the_function_budget_names_its_body_size() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    catalog.add_stmt(run, "a();").expect("stmt");
    catalog.add_stmt(run, "b();").expect("stmt");
    let policy = Policy {
        max_fn_lines: 1,
        ..fixture::policy()
    };

    let found = rendered_violations(&catalog, &policy);

    assert_eq!(found[0].code, Code::FnBudget);
    assert_eq!(found[0].id, Some(run));
    assert_eq!(found[0].size, Some(2));
}

#[test]
fn a_module_without_a_preamble_is_refused_when_the_policy_requires_one() {
    let mut catalog = fixture::catalog();
    fixture::module(&mut catalog, "src/main.rs");
    fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = Policy {
        require_preamble: true,
        ..fixture::policy()
    };

    let found = rendered_violations(&catalog, &policy);

    let preamble: Vec<&Violation> = found.iter().filter(|v| v.code == Code::Preamble).collect();
    assert_eq!(preamble.len(), 1);
    assert!(preamble[0].remedy.contains("src/a.rs"), "{}", preamble[0].remedy);
}

#[test]
fn a_preamble_satisfies_the_preamble_rule() {
    let mut catalog = fixture::catalog();
    // The crate root is a file like any other, so it needs a preamble too: the
    // rule has no special case, and a special case is how a root ends up empty.
    fixture::module(&mut catalog, "src/main.rs");
    fixture::module(&mut catalog, "src/a.rs");
    fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = Policy {
        require_preamble: true,
        ..fixture::policy()
    };

    assert!(rendered_violations(&catalog, &policy).is_empty());
}

#[test]
fn an_opaque_file_is_not_a_function_body_budget() {
    use crate::core::model::element::BodyMode;

    let mut catalog = fixture::catalog();
    let id = fixture::module(&mut catalog, "src/main.rs");
    for text in ["a();\n", "b();\n", "c();\n"] {
        catalog.add_stmt(id, text).expect("stmt");
    }
    catalog.find_mut(id).expect("module").mode = BodyMode::Opaque;
    let policy = Policy {
        ceiling: 1000,
        max_fn_lines: 1,
        ..fixture::policy()
    };

    let found = rendered_violations(&catalog, &policy);

    assert!(!found.iter().any(|violation| violation.code == Code::FnBudget), "{found:?}");
}

#[test]
fn an_empty_function_costs_no_function_budget() {
    let mut catalog = fixture::catalog();
    fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = fixture::policy();

    let found = rendered_violations(&catalog, &policy);

    assert!(!found.iter().any(|violation| violation.code == Code::FnBudget), "{found:?}");
}
