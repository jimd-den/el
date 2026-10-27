// A test added without a body panics with todo!() under cargo test,
// so the red step needs no body at all: add the test, see it fail.

use crate::program::{el, el_stdin, fresh, ok};
use std::process::Command;

#[test]
fn a_test_without_a_body_fails_by_default() {
    let root = fresh("test-no-body");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/util.rs"]), "add mod");
    let answer = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&answer, "add answer");
    let test = el(&hello, &["add", "test", "answer_returns_42", "--in", "src/util.rs", "--sig", "fn answer_returns_42()"]);
    ok(&test, "add test without a body");

    let run = Command::new(env!("CARGO"))
        .current_dir(&hello)
        .args(["test", "-q"])
        .output()
        .expect("cargo test");
    assert!(!run.status.success(), "a bodyless test must fail, not pass vacuously");

    let test_id = String::from_utf8_lossy(&test.stdout).trim().to_string();
    let shown = el(&hello, &["show", &test_id, "--source"]);
    ok(&shown, "el show test --source");
    let source = String::from_utf8_lossy(&shown.stdout);
    assert!(source.contains("todo!()"), "bodyless test renders no todo:\n{source}");
}
