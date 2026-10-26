// FR-26 and FR-73: the policy's ceilings are enforced at L0, with the offending
// element and its size named, and the budget is computed per statement so the
// refusal can suggest a seam instead of stating a wall.
//
// The boundary is `plan` for the reason given in
// `model_violation_writes_nothing.rs`: no store exists yet, so the refusal is
// proved where it happens rather than where it would be reported.

use el::core::model::catalog::Catalog;
use el::core::model::change::Change;
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
    catalog.add_stmt(run, "first();").expect("stmt");
    catalog.add_stmt(run, "second();").expect("stmt");
    (catalog, run)
}

fn policy() -> Policy {
    Policy {
        require_preamble: false,
        ..Policy::default()
    }
}

fn grow() -> Change {
    Change::AddStmt {
        owner: ElId(1),
        text: "third();".to_string(),
    }
}

#[test]
fn a_statement_that_would_break_the_file_ceiling_is_refused() {
    let (catalog, _) = catalog();
    let fits = plan(&catalog, &ask(&grow(), &policy())).expect("fits the default ceiling");
    let lines = fits
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

    let refused = plan(&catalog, &ask(&grow(), &tight)).expect_err("over the ceiling");

    assert_eq!(refused.first().code, Code::Ceiling);
    assert_eq!(refused.first().size, Some(lines));
}

#[test]
fn a_ceiling_refusal_names_the_statement_ids_that_could_be_extracted() {
    let (catalog, _) = catalog();
    let tight = Policy {
        ceiling: 4,
        ..policy()
    };

    let refused = plan(&catalog, &ask(&grow(), &tight)).expect_err("over the ceiling");
    let remedy = &refused.first().remedy;

    assert!(remedy.contains("extract statement el-"), "{remedy}");
}

#[test]
fn a_body_past_the_function_budget_is_refused_with_its_size() {
    let (catalog, _) = catalog();
    let tight = Policy {
        max_fn_lines: 1,
        ..policy()
    };

    let refused = plan(&catalog, &ask(&grow(), &tight)).expect_err("three lines in a body of one");
    let budget = refused
        .iter()
        .find(|violation| violation.code == Code::FnBudget)
        .expect("a function budget violation");

    assert_eq!(budget.id, Some(ElId(1)));
    assert_eq!(budget.size, Some(3));
}

#[test]
fn a_body_within_the_function_budget_is_allowed() {
    let (catalog, _) = catalog();
    let roomy = Policy {
        max_fn_lines: 4,
        ..policy()
    };

    assert!(plan(&catalog, &ask(&grow(), &roomy)).is_ok());
}
