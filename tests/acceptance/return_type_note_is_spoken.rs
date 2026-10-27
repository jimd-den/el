// The return-type note: a fn added without --sig and without
// --returns, whose body ends in a value, is told it may need a return
// type. It is a note, not a refusal, and it stays silent when the
// body ends in a semicolon or a return type is declared.

use crate::program::{el, el_stdin, fresh, ok};

fn combined(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn crate_with_mod() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = fresh("return-note");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "el new");
    ok(&el(&hello, &["add", "mod", "src/util.rs"]), "add mod");
    (root, hello)
}

#[test]
fn a_tail_body_without_a_return_type_is_told_so() {
    let (_root, hello) = crate_with_mod();
    let added = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--body", "-"],
        "0",
    );
    assert_eq!(
        added.status.code(),
        Some(3),
        "a tail body without a return type must be gate-rejected"
    );

    let text = combined(&added);
    assert!(text.contains("declare a return type"), "no note was spoken:\n{text}");
}

#[test]
fn a_test_with_a_tail_body_is_told_so() {
    let (_root, hello) = crate_with_mod();
    let answer = el_stdin(
        &hello,
        &[
            "add", "fn", "answer", "--in", "src/util.rs", "--returns", "u32", "--body", "-",
        ],
        "42",
    );
    ok(&answer, "add answer");
    let added = el_stdin(
        &hello,
        &["add", "test", "answer_returns", "--in", "src/util.rs", "--body", "-"],
        "answer()",
    );
    assert_eq!(
        added.status.code(),
        Some(3),
        "a test with a non-() tail must be gate-rejected like a fn"
    );

    let text = combined(&added);
    assert!(text.contains("declare a return type"), "no note was spoken:\n{text}");
}

#[test]
fn a_fast_add_with_a_tail_body_is_told_so() {
    let (_root, hello) = crate_with_mod();
    let added = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--body", "-", "--fast"],
        "0",
    );
    ok(&added, "add fn --fast with a tail body");

    let text = combined(&added);
    assert!(text.contains("declare a return type"), "no note was spoken:\n{text}");
}

#[test]
fn a_body_ending_in_a_semicolon_gets_no_note() {
    let (_root, hello) = crate_with_mod();
    let added = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "src/util.rs", "--body", "-"],
        "println!(\"hi\");",
    );
    ok(&added, "add fn with a semicolon body");

    let text = combined(&added);
    assert!(!text.contains("declare a return type"), "a note was spoken:\n{text}");
}

#[test]
fn a_declared_return_type_gets_no_note() {
    let (_root, hello) = crate_with_mod();
    let added = el_stdin(
        &hello,
        &[
            "add", "fn", "answer", "--in", "src/util.rs", "--returns", "u32", "--body", "-",
        ],
        "0",
    );
    ok(&added, "add fn with --returns");

    let text = combined(&added);
    assert!(!text.contains("declare a return type"), "a note was spoken:\n{text}");
}
