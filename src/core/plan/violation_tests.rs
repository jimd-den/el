// Tests for `violation`; sibling of the module they exercise, so the module
// stays small enough to read in one sitting (§4.4).

use super::*;

#[test]
fn every_rule_has_its_own_code() {
    let spelled: Vec<&str> = Code::all().iter().map(|code| code.as_str()).collect();

    assert_eq!(spelled.len(), 11);
    assert_eq!(spelled.len(), spelled.iter().collect::<std::collections::HashSet<_>>().len());
}

#[test]
fn a_code_says_which_tier_it_belongs_to() {
    for code in Code::all() {
        assert!(code.as_str().starts_with("model."), "{code:?}");
    }
}

#[test]
fn a_violation_names_its_element_and_its_size() {
    let violation = Violation::at(Code::Ceiling, ElId(7), "extract el-0009").sized(201);

    assert_eq!(
        violation.to_string(),
        "model.ceiling el-0007 (201): extract el-0009"
    );
}

#[test]
fn a_violation_without_an_element_still_reads() {
    assert_eq!(
        Violation::new(Code::DepDenied, "remove the dependency").to_string(),
        "model.dep_denied: remove the dependency"
    );
}

#[test]
fn an_empty_set_is_not_a_refusal() {
    assert_eq!(Violations::new(Vec::new()), None);
}

#[test]
fn a_set_reports_its_codes_in_order() {
    let set = Violations::new(vec![
        Violation::new(Code::UnknownRef, "a"),
        Violation::new(Code::BadIdent, "b"),
    ])
    .expect("non-empty");

    assert_eq!(set.codes(), vec![Code::UnknownRef, Code::BadIdent]);
    assert_eq!(set.first().code, Code::UnknownRef);
}
