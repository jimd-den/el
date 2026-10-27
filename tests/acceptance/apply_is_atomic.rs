// `el apply` is `batch`'s script sibling: one line is one change, every line
// rides one transaction, and a failure anywhere applies none of them. A plan
// refusal names the line; a gate rejection restores and journals one attempt.

use std::fs;
use std::path::Path;

use crate::program::{el, el_stdin, fresh, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn snippet(hello: &Path, name: &str, body: &str) {
    fs::write(hello.join(name), body).expect("write snippet");
}

#[test]
fn apply_runs_many_change_kinds_in_one_transaction() {
    let root = fresh("apply-many");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    snippet(&hello, "twice.snippet", "x * 2\n");
    snippet(&hello, "main.snippet", "println!(\"{}\", extra::twice(21));\n");
    let script = "# a module and its item, then the program's main\n\
                  add mod src/extra.rs\n\
                  add fn twice --in src/extra.rs --vis pub --sig 'fn twice(x: u32) -> u32' --body twice.snippet\n\
                  add fn main --sig 'fn main()' --body main.snippet\n";
    fs::write(hello.join("apply.el"), script).expect("write script");

    let output = el(&hello, &["apply", "apply.el"]);

    ok(&output, "apply");
    let tree = text(&el(&hello, &["tree"]));
    assert!(tree.contains("extra::fn/twice"), "tree:\n{tree}");
    let run = text(&el(&hello, &["run"]));
    assert!(run.contains("42"), "run:\n{run}");
}

#[test]
fn a_plan_failure_names_its_line_and_applies_nothing() {
    let root = fresh("apply-plan-failure");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    snippet(&hello, "twice.snippet", "x * 2\n");
    let script = "add mod src/extra.rs\n\
                  add fn twice --in src/extra.rs --sig 'fn twice(x: u32) -> u32' --body twice.snippet\n\
                  add mod src/extra.rs\n\
                  rm el-0001\n";

    let output = el_stdin(&hello, &["apply"], script);

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(text(&output).contains("apply line 3"), "{}", text(&output));
    let tree = text(&el(&hello, &["tree"]));
    assert!(!tree.contains("extra::fn/twice"), "nothing applied:\n{tree}");
    assert!(tree.contains("mod el-0001"), "the rm did not run either:\n{tree}");
}

#[test]
fn a_gate_rejection_restores_and_journals_one_attempt() {
    let root = fresh("apply-gate-rejection");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    snippet(&hello, "broken.snippet", "undefined_fn();\n");
    let script = "add mod src/extra.rs\n\
                  add fn broken --in src/extra.rs --sig 'fn broken()' --body broken.snippet\n";

    let output = el_stdin(&hello, &["apply"], script);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    let tree = text(&el(&hello, &["tree"]));
    assert!(!tree.contains("extra"), "nothing applied:\n{tree}");
    let log = text(&el(&hello, &["log"]));
    assert!(log.contains("rejected:1"), "log:\n{log}");
}

#[test]
fn a_script_cannot_address_an_id_it_has_not_made_yet() {
    let root = fresh("apply-future-id");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let script = "add fn one --sig 'fn one()'\n\
                  edit el-0009 --rename two\n";

    let output = el_stdin(&hello, &["apply"], script);

    assert_eq!(output.status.code(), Some(6), "{}", text(&output));
    assert!(text(&output).contains("apply line 2"), "{}", text(&output));
    let tree = text(&el(&hello, &["tree"]));
    assert!(!tree.contains("fn one"), "nothing applied:\n{tree}");
}

#[test]
fn a_script_line_cannot_read_stdin() {
    let root = fresh("apply-stdin-body");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el_stdin(&hello, &["apply"], "add fn one --sig 'fn one()' --body -\n");

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(text(&output).contains("apply line 1"), "{}", text(&output));
}

#[test]
fn a_one_line_script_names_its_line() {
    let root = fresh("apply-one-line");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el_stdin(&hello, &["apply"], "add mod src/main.rs\n");

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(text(&output).contains("apply line 1"), "{}", text(&output));
}

#[test]
fn an_unterminated_quote_names_the_line_once() {
    let root = fresh("apply-unterminated");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el_stdin(&hello, &["apply"], "add fn one --sig 'fn one(\n");

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    let shown = text(&output);
    assert!(!shown.contains("apply line 1: apply line 1"), "{shown}");
}
