// ADR-0033: the add stmt subject is the owning function, and the
// refusals plus the skill page must say so — the bench model looped
// for ~45 calls between "needs a name" and "resolve a name with
// `el find`", neither of which names the rule.

use crate::program::{self, el, el_stdin, ok};

fn out(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn add_fn(hello: &std::path::Path) -> String {
    ok(&el(hello, &["add", "mod", "src/util.rs"]), "add mod");
    let added = el_stdin(
        hello,
        &[
            "add", "fn", "double", "--in", "src/util.rs", "--sig",
            "fn double(x: u32) -> u32", "--body", "-", "--fast",
        ],
        "x * 2\n",
    );
    ok(&added, "add fn");
    String::from_utf8_lossy(&added.stdout).trim().to_string()
}

#[test]
fn add_stmt_without_a_subject_names_the_owning_function() {
    let built = program::build("stmt-no-subject");
    let _ = add_fn(&built.hello);

    let refused = el_stdin(
        &built.hello,
        &["add", "stmt", "--body", "-", "--at", "0"],
        "let _ = x;\n",
    );
    assert!(!refused.status.success(), "a missing subject must refuse");
    // Usage errors print to stderr, unlike report refusals.
    let text = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(
        text.contains("owning function"),
        "the refusal names the owner rule: {text}"
    );
    assert!(
        !text.contains("needs a name"),
        "the refusal must not say 'name': {text}"
    );
}

#[test]
fn add_stmt_with_a_name_subject_names_the_owner_rule() {
    let built = program::build("stmt-name-subject");
    let _ = add_fn(&built.hello);

    let refused = el_stdin(
        &built.hello,
        &["add", "stmt", "let_underscore_x", "--body", "-", "--at", "0"],
        "let _ = x;\n",
    );
    assert!(!refused.status.success(), "a name subject must refuse");
    let text = out(&refused);
    assert!(
        text.contains("owning function"),
        "the refusal names the owner rule: {text}"
    );
    assert!(
        !text.contains("el find let_underscore_x"),
        "the refusal must not send the model hunting with find: {text}"
    );
}

#[test]
fn skill_page_shows_the_stmt_form() {
    let built = program::build("stmt-skill");
    let skill = el(&built.hello, &["skill"]);
    ok(&skill, "skill");
    let text = out(&skill);
    assert!(text.contains("add stmt"), "the page shows add stmt: {text}");
    assert!(
        text.contains("owning function"),
        "the page states the owner rule: {text}"
    );
}
