// `el tdd start` captures a strict run: the target test identity, the
// test lock, the production baseline, and the config lock, written to
// `.el/runs/<run-id>/record`, with `.el/runs/active` pointing at it.
// A second start while a run is active is refused.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn find_test_id(hello: &std::path::Path, name: &str) -> String {
    let output = el(hello, &["find", name]);
    ok(&output, "find test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().unwrap_or_else(|| panic!("no find output: {stdout}"));
    first.split_whitespace().next().unwrap_or_else(|| panic!("no id in: {first}")).to_string()
}

#[test]
fn a_start_records_the_run_and_a_second_start_is_refused() {
    let root = fresh("tdd-start");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let output = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&output, "add answer");
    let output = el_stdin(
        &hello,
        &["add", "test", "answer_returns_42", "--in", "main", "--body", "-"],
        "assert_eq!(answer(), 42);",
    );
    ok(&output, "add test");
    let test_id = find_test_id(&hello, "answer_returns_42");

    let output = el(&hello, &["tdd", "start", "--target", &test_id, "--json"]);
    ok(&output, "tdd start");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("start"));
    assert_eq!(data.get("target").and_then(|v| v.as_str()), Some(test_id.as_str()));
    let run = data
        .get("run")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("no run: {json:?}"))
        .to_string();
    let record = std::fs::read_to_string(hello.join(".el").join("runs").join(&run).join("record"))
        .unwrap_or_else(|e| panic!("no record file for {run}: {e}"));
    let record_json = el::core::text::json::parse(record.trim())
        .unwrap_or_else(|| panic!("record is not JSON: {record}"));
    assert_eq!(record_json.get("phase").and_then(|v| v.as_str()), Some("start"));
    assert_eq!(record_json.get("target").and_then(|v| v.as_str()), Some(test_id.as_str()));
    let active = std::fs::read_to_string(hello.join(".el").join("runs").join("active"))
        .unwrap_or_else(|e| panic!("no active marker: {e}"));
    assert_eq!(active.trim(), run);

    let again = el(&hello, &["tdd", "start", "--target", &test_id, "--json"]);
    assert!(!again.status.success(), "a second start must be refused");
}
