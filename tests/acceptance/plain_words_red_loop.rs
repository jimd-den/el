// The red-green loop spoken entirely in plain words: the fn is added
// with --returns, the test with no --sig at all, red is an assertion
// failure, green follows an edit. No --sig anywhere, no hand-edited
// .rs file.

use crate::program::{el, el_stdin, fresh, ok};

fn cargo_test(hello: &std::path::Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .current_dir(hello)
        .args(["test", "-q"])
        .output()
        .expect("cargo test runs")
}

#[test]
fn the_red_green_loop_works_in_plain_words() {
    let root = fresh("plain-words-red-loop");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/util.rs"]), "add mod");

    let added = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--returns", "u32", "--body", "-"],
        "0",
    );
    ok(&added, "add fn in plain words");
    let answer_id = String::from_utf8_lossy(&added.stdout).trim().to_string();

    let test = el_stdin(
        &hello,
        &["add", "test", "answer_returns_42", "--in", "src/util.rs", "--body", "-"],
        "assert_eq!(answer(), 42);",
    );
    ok(&test, "add test in plain words");

    let red = cargo_test(&hello);
    assert!(!red.status.success(), "red: cargo test must fail first");
    let red_text = String::from_utf8_lossy(&red.stdout).into_owned()
        + &String::from_utf8_lossy(&red.stderr);
    assert!(red_text.contains("FAILED"), "red output must name the failure");

    let edited = el_stdin(&hello, &["edit", &answer_id, "--body", "-"], "42");
    ok(&edited, "edit answer");

    let green = cargo_test(&hello);
    assert!(green.status.success(), "green: cargo test must pass after the edit");
}
