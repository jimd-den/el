// The tests for `plan`'s one structural side effect: a move into a module that
// does not exist yet. They live apart so `mod_tests` stays a readable list of the
// plan boundary rather than a place where every rule's exception accumulates.

use super::*;
use crate::core::model::kind::Kind;
use crate::core::plan::fixture;

fn ask<'a>(change: &'a Change, policy: &'a policy::Policy) -> Request<'a> {
    Request {
        change,
        policy,
        crate_name: "demo",
    }
}

#[test]
fn a_move_to_a_new_module_creates_the_module_with_a_preamble() {
    let mut catalog = fixture::catalog();
    let run = fixture::function(&mut catalog, "src/a.rs", "fn run()");
    let policy = crate::core::plan::policy::Policy {
        require_preamble: true,
        ..fixture::policy()
    };
    let change = Change::MoveElement {
        id: run,
        module: "src/core/gate.rs".to_string(),
    };

    let outcome = plan(&catalog, &ask(&change, &policy)).expect("mv creates its module");

    assert!(outcome.catalog.elements.iter().any(|element| {
        element.kind == Kind::Module && element.module == "src/core/gate.rs"
    }));
    assert!(
        outcome
            .projection
            .file("src/core/gate.rs")
            .expect("moved file")
            .text
            .contains("//"),
    );
}

