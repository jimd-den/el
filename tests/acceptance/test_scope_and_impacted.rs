// `el test` scopes execution and reports the fallback honestly:
// `--impacted` runs the full suite when no dependency metadata exists,
// and says so in `data.fallback` instead of pretending it was impacted.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn crate_with_tests() -> std::path::PathBuf {
    let root = fresh("test-scope-impacted");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let answer = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&answer, "add answer");
    let main = el_stdin(
        &hello,
        &["add", "fn", "main", "--in", "main", "--sig", "fn main()", "--body", "-"],
        "",
    );
    ok(&main, "add main");
    let test = el_stdin(
        &hello,
        &["add", "test", "answer_returns_42", "--in", "main", "--body", "-"],
        "assert_eq!(answer(), 42);",
    );
    ok(&test, "add test");
    hello
}

#[test]
fn test_scope_filters_execution_and_records_the_scope() {
    let hello = crate_with_tests();
    let output = el(&hello, &["test", "--scope", "answer", "--json"]);
    ok(&output, "test --scope");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("passed"));
    let tests = data.get("tests").and_then(|v| v.as_array()).unwrap();
    assert!(tests.iter().any(|test| test
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("answer_returns_42")));
    let evidence = json.get("evidence").unwrap_or_else(|| panic!("no evidence: {json:?}"));
    assert_eq!(evidence.get("kind").and_then(|v| v.as_str()), Some("test"));
    let scope = evidence.get("scope").unwrap_or_else(|| panic!("no scope: {json:?}"));
    assert_eq!(scope.get("requested").and_then(|v| v.as_str()), Some("answer"));
}

#[test]
fn test_impacted_falls_back_to_all_and_says_so() {
    let hello = crate_with_tests();
    let output = el(&hello, &["test", "--impacted", "--json"]);
    ok(&output, "test --impacted");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("passed"));
    assert_eq!(data.get("fallback").and_then(|v| v.as_str()), Some("all"));
}
