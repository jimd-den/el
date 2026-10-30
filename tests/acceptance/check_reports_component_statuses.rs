// `el check` reports every component's status explicitly: a passing
// check leaves test, fmt and lint `not_run` — never `passed` merely
// because they have not run.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn add_answer(hello: &std::path::Path, body: &str) {
    let output = el_stdin(
        hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        body,
    );
    ok(&output, "add answer");
}

/// `el new` seeds a comment-only main, which is a real E0601 until a
/// main fn lands; a passing check needs one.
fn add_main(hello: &std::path::Path) {
    let output = el_stdin(
        hello,
        &["add", "fn", "main", "--in", "main", "--sig", "fn main()", "--body", "-"],
        "",
    );
    ok(&output, "add main");
}

#[test]
fn a_passing_check_leaves_the_other_components_not_run() {
    let root = fresh("check-statuses-pass");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_answer(&hello, "42");
    add_main(&hello);

    let output = el(&hello, &["check", "--json"]);
    ok(&output, "check --json");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("check").and_then(|v| v.as_str()), Some("passed"));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("not_run"));
    assert_eq!(data.get("fmt").and_then(|v| v.as_str()), Some("not_run"));
    assert_eq!(data.get("lint").and_then(|v| v.as_str()), Some("not_run"));
    let evidence = json.get("evidence").unwrap_or_else(|| panic!("no evidence: {json:?}"));
    assert_eq!(evidence.get("kind").and_then(|v| v.as_str()), Some("check"));
    assert_eq!(evidence.get("result").and_then(|v| v.as_str()), Some("passed"));
    let command = evidence.get("command").and_then(|v| v.as_array()).unwrap();
    assert_eq!(command[0].as_str(), Some("cargo"));
}

#[test]
fn a_failing_check_still_leaves_test_not_run() {
    let root = fresh("check-statuses-fail");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    // A gated add would refuse the type error at add time, so the
    // broken tree is built with `--fast`: the gate is bypassed by
    // design, and `check` is what reports the breakage.
    let broken = el_stdin(
        &hello,
        &["--fast", "add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "\"not a number\"",
    );
    ok(&broken, "add broken answer with --fast");
    add_main(&hello);

    let output = el(&hello, &["check", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("check").and_then(|v| v.as_str()), Some("failed"));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("not_run"));
}
