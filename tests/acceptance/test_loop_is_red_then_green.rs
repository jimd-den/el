// The TDD loop through el verbs only: a test fails before the
// implementation exists and passes after, with no hand-edited .rs file.
// The red step must itself commit: the gate is cargo check, which does
// not run tests, so a failing test is not a gate rejection.

use crate::program::{el, el_stdin, fresh, ok};
use std::path::Path;
use std::process::{Command, Output};

fn cargo_test(cwd: &Path) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(cwd)
        .args(["test", "-q"])
        .output()
        .expect("cargo test")
}

fn id_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn a_test_fails_before_the_implementation_and_passes_after() {
    let root = fresh("test-loop");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/util.rs"]), "add mod");
    let answer = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--sig", "fn answer() -> u32", "--body", "-"],
        "0",
    );
    ok(&answer, "add answer");
    let test = el_stdin(
        &hello,
        &[
            "add", "test", "answer_returns_42", "--in", "src/util.rs", "--sig",
            "fn answer_returns_42()", "--body", "-",
        ],
        "assert_eq!(answer(), 42);",
    );
    ok(&test, "add test must commit: red is not a gate rejection");
    let test_id = id_of(&test);

    let red = cargo_test(&hello);
    assert!(!red.status.success(), "red step must fail before the implementation");

    let edit = el_stdin(&hello, &["edit", &id_of(&answer), "--body", "-"], "42");
    ok(&edit, "edit answer");

    let green = cargo_test(&hello);
    assert!(green.status.success(), "green step must pass after the implementation");

    let shown = el(&hello, &["show", &test_id, "--source"]);
    ok(&shown, "el show test --source");
    let source = String::from_utf8_lossy(&shown.stdout);
    assert!(source.contains("#[test]"), "rendered test carries no attribute:\n{source}");
}
