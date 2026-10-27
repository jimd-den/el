// A subject that is not an element id: the refusal names the command that
// resolves a name, so an agent that addressed by name does not have to guess
// how to get the id it actually needs.

use super::*;

#[test]
fn a_name_instead_of_an_id_names_the_find_command() {
    let error = parse_id("greet").expect_err("a name is not an id");

    assert_eq!(error.code, "notfound.element");
    assert!(
        error.message.contains("el find greet"),
        "refusal must name the resolution:\n{}",
        error.message
    );
}

#[test]
fn a_spaced_subject_gets_no_find_command() {
    let error = parse_id("two words").expect_err("spaced text is not an id");

    assert!(
        !error.message.contains("el find"),
        "a spaced subject is not one find argument:\n{}",
        error.message
    );
}
