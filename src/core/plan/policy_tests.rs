// Tests for `policy`; sibling of the module they exercise, so the module
// stays small enough to read in one sitting (§4.4).

use super::*;

#[test]
fn no_policy_yields_the_project_defaults() {
    let policy = Policy::parse("").expect("empty is valid");

    assert_eq!(policy, Policy::default());
    assert_eq!(policy.ceiling, 200);
    assert_eq!(policy.max_fn_lines, 15);
    assert!(policy.require_preamble);
    assert!(policy.forbids("unsafe"));
    assert!(!policy.allows_dep("serde"));
    assert_eq!(policy.gate, "check");
}

#[test]
fn a_file_overrides_only_what_it_names() {
    let policy = Policy::parse("ceiling = 120\n").expect("valid");

    assert_eq!(policy.ceiling, 120);
    assert_eq!(policy.max_fn_lines, 15);
    assert!(policy.require_preamble);
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let text = "\n# the ceiling\nceiling = 90   # trailing\n\n";

    assert_eq!(Policy::parse(text).expect("valid").ceiling, 90);
}

#[test]
fn an_unknown_key_names_its_line() {
    let error = Policy::parse("ceiling = 90\nceilng = 80\n").expect_err("typo");

    assert_eq!(error.line, 2);
    assert_eq!(error.reason, "unknown key");
    assert!(error.to_string().contains("el.policy line 2"), "{error}");
}

#[test]
fn a_non_numeric_ceiling_names_its_line() {
    let error = Policy::parse("ceiling = lots\n").expect_err("not a number");

    assert_eq!(error.line, 1);
    assert!(error.to_string().contains("must be a number"), "{error}");
}

#[test]
fn a_line_without_an_equals_sign_names_its_line() {
    let error = Policy::parse("\nceiling 200\n").expect_err("no `=`");

    assert_eq!(error.line, 2);
    assert!(error.to_string().contains("expected `key = value`"), "{error}");
}

#[test]
fn deps_none_allows_nothing() {
    assert!(!Policy::parse("deps = none\n").expect("valid").allows_dep("serde"));
}

#[test]
fn deps_lists_what_is_allowed() {
    let policy = Policy::parse("deps = serde, rand\n").expect("valid");

    assert!(policy.allows_dep("serde"));
    assert!(policy.allows_dep("rand"));
    assert!(!policy.allows_dep("tracing"));
}

#[test]
fn forbid_is_a_list_and_can_be_emptied() {
    let policy = Policy::parse("forbid = unsafe, static mut\n").expect("valid");
    assert!(policy.forbids("static mut"));
    assert!(!policy.forbids("loop"));

    assert_eq!(Policy::parse("forbid = none\n").expect("valid").forbid, Vec::<String>::new());
}

#[test]
fn an_unknown_gate_word_names_its_line() {
    let error = Policy::parse("\ngate = fast\n").expect_err("not a tier");

    assert_eq!(error.line, 2);
    assert!(error.to_string().contains("must be model"), "{error}");
}

#[test]
fn a_preamble_requirement_can_be_turned_off() {
    assert!(!Policy::parse("require_preamble = false\n").expect("valid").require_preamble);
}
