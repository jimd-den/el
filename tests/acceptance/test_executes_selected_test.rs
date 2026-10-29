// `el test` executes tests and reports execution, not compilation: a
// passing suite is ok, a failing test is a test failure (not a compile
// error), and `--target` runs one test element by id.

use crate::program::{el, el_stdin, fresh, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn id_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn add_fn_and_tests(hello: &std::path::Path) -> (String, String) {
    let answer = el_stdin(
        hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&answer, "add answer");
    let good = el_stdin(
        hello,
        &[
            "add", "test", "answer_is_42", "--in", "main", "--sig", "fn answer_is_42()",
            "--body", "-",
        ],
        "assert_eq!(answer(), 42);",
    );
    ok(&good, "add passing test");
    let bad = el_stdin(
        hello,
        &[
            "add", "test", "answer_is_43", "--in", "main", "--sig", "fn answer_is_43()",
            "--body", "-",
        ],
        "assert_eq!(answer(), 43);",
    );
    ok(&bad, "add failing test");
    (id_of(&good), id_of(&bad))
}

#[test]
fn test_runs_the_suite_and_reports_execution() {
    let root = fresh("test-suite");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_fn_and_tests(&hello);

    let output = el(&hello, &["test"]);
    let out = text(&output);
    assert_eq!(output.status.code(), Some(3), "one test fails:\n{out}");
    assert!(out.contains("1 passed"), "passed count:\n{out}");
    assert!(out.contains("1 failed"), "failed count:\n{out}");
    assert!(out.contains("answer_is_42"), "executed tests:\n{out}");
    assert!(out.contains("answer_is_43"), "executed tests:\n{out}");
    assert!(!out.contains("error["), "a test failure is not a compile error:\n{out}");
}

#[test]
fn test_target_runs_one_test_by_id() {
    let root = fresh("test-target");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let (good, bad) = add_fn_and_tests(&hello);

    let output = el(&hello, &["test", "--target", &good]);
    let out = text(&output);
    assert_eq!(output.status.code(), Some(0), "passing target:\n{out}");
    assert!(out.contains("answer_is_42"), "target ran:\n{out}");
    assert!(!out.contains("answer_is_43"), "only the target runs:\n{out}");

    let output = el(&hello, &["test", "--target", &bad]);
    let out = text(&output);
    assert_eq!(output.status.code(), Some(3), "failing target:\n{out}");
    assert!(out.contains("answer_is_43"), "target ran:\n{out}");
    assert!(out.contains("failed"), "failure reported:\n{out}");
}

#[test]
fn test_json_carries_passed_and_failed_counts() {
    let root = fresh("test-json");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_fn_and_tests(&hello);

    let output = el(&hello, &["test", "--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"));
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {stdout}"));
    assert_eq!(data.get("passed").and_then(|v| v.as_number()), Some(1));
    assert_eq!(data.get("failed").and_then(|v| v.as_number()), Some(1));
}
