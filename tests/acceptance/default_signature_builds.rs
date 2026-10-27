// FR-6/FR-23: the head is produced, not left to the user. A function typed
// without `--sig` is still a declaration; a kind whose head needs a type the tool
// cannot invent is refused before anything is staged.

use crate::program::{el, el_stdin, fresh, ok};
use std::fs;

#[test]
fn a_function_without_a_sig_is_still_a_declaration() {
    let root = fresh("default-sig");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");

    let added = el_stdin(
        &hello,
        &["add", "fn", "main", "--body", "-"],
        "println!(\"hi\");",
    );

    ok(&added, "add fn main");
    let run = el(&hello, &["run"]);
    ok(&run, "run");
    assert!(String::from_utf8_lossy(&run.stdout).contains("hi"));
}

#[test]
fn a_const_without_a_sig_is_refused_before_it_is_written() {
    let root = fresh("default-sig-const");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    let before = fs::read_to_string(hello.join("src/main.rs")).expect("main");

    let output = el(&hello, &["add", "const", "LIMIT"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(hello.join("src/main.rs")).expect("main"),
        before
    );
}
