// A refusal that leaves the user with nowhere to go is a dead end, not a
// diagnostic. Every failure — a parse refusal, a missing crate, an unknown id, a
// gate rejection — prints a next command, and that command is checked against the
// real parser: a hint that does not parse is worse than no hint.

use crate::program::{el, el_stdin, fresh, ok};

fn next(hint: &str) -> Vec<String> {
    hint.split_whitespace().skip(1).map(str::to_string).collect()
}

fn parses(hint: &str) {
    assert!(hint.starts_with("el "), "hint is not a command: {hint:?}");
    assert!(
        el::cli::parse(&next(hint)).is_ok(),
        "hint does not parse: {hint:?}"
    );
}

fn hint_from_json(output: &std::process::Output) -> String {
    let text = String::from_utf8_lossy(&output.stdout);
    let json = el::core::text::json::parse(text.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {text}"));
    let hint = json
        .get("error")
        .and_then(|error| error.get("hint"))
        .and_then(|hint| hint.as_str())
        .unwrap_or_else(|| panic!("no error.hint: {text}"));
    hint.to_string()
}

#[test]
fn a_parse_refusal_names_the_help_page() {
    let root = fresh("hint-parse");

    let output = el(&root, &["frobnicate"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let hint = stderr
        .lines()
        .find_map(|line| line.strip_prefix("next: "))
        .unwrap_or_else(|| panic!("no next line: {stderr}"));
    parses(hint);
}

#[test]
fn a_missing_crate_names_how_to_make_one() {
    let root = fresh("hint-crate");

    let output = el(&root, &["ls", "--json"]);

    assert_eq!(output.status.code(), Some(4));
    let hint = hint_from_json(&output);
    assert_eq!(hint, "el new hello");
    parses(&hint);
}

#[test]
fn an_unknown_id_names_the_listing() {
    let root = fresh("hint-id");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el(&hello, &["show", "el-9999", "--json"]);

    assert_eq!(output.status.code(), Some(6));
    let hint = hint_from_json(&output);
    assert_eq!(hint, "el ls");
    parses(&hint);
}

#[test]
fn a_gate_rejection_names_the_diff() {
    let root = fresh("hint-gate");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el_stdin(
        &hello,
        &["add", "fn", "broken", "--sig", "fn broken() -> u32", "--body", "-"],
        "\"not a number\"\n",
    );

    assert_eq!(
        output.status.code(),
        Some(3),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let hint = text
        .lines()
        .find_map(|line| line.strip_prefix("next: "))
        .unwrap_or_else(|| panic!("no next line: {text}"));
    assert_eq!(hint, "el diff");
    parses(hint);
}
