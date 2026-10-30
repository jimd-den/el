// `el tdd green` proves the fix: the target test passes, the test
// lock is exactly as red froze it, and config is unchanged.
// Production changes are the point of green, so the green state
// snapshot replaces the start baseline for verify to check against.
// A changed test is refused with `EL_TEST_LOCKED`; a target that
// still fails is refused with `EL_GATE_FAILED`.
// (Skipped tests cannot be expressed through el: the model renders
// every test as `#[test] fn` with no attribute slot, so the green
// weakening check is defense in depth, covered at unit level.)

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn find_id(hello: &std::path::Path, name: &str) -> String {
    let output = el(hello, &["find", name]);
    ok(&output, "find");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().unwrap_or_else(|| panic!("no find output: {stdout}"));
    first.split_whitespace().next().unwrap_or_else(|| panic!("no id in: {first}")).to_string()
}

/// A crate at red: answer returns 42, the test asserts 43, and
/// `tdd red` has proved the failure.
fn crate_at_red(root: &std::path::Path) -> std::path::PathBuf {
    let hello = root.join("hello");
    ok(&el(root, &["new", "hello"]), "new");
    let output = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&output, "add answer");
    let output = el_stdin(
        &hello,
        &["add", "test", "answer_returns_42", "--in", "main", "--body", "-"],
        "assert_eq!(answer(), 43);",
    );
    ok(&output, "add test");
    let test_id = find_id(&hello, "answer_returns_42");
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "tdd start");
    ok(&el(&hello, &["tdd", "red"]), "tdd red");
    hello
}

#[test]
fn green_accepts_a_passing_target_with_unchanged_tests() {
    let root = fresh("tdd-green-accepts");
    let hello = crate_at_red(&root);
    let answer_id = find_id(&hello, "answer");
    let output = el_stdin(&hello, &["edit", &answer_id, "--body", "-"], "43");
    ok(&output, "edit answer to 43");

    let output = el(&hello, &["tdd", "green", "--json"]);
    ok(&output, "tdd green");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("green"));
    let run = data.get("run").and_then(|v| v.as_str()).unwrap().to_string();
    let record = std::fs::read_to_string(hello.join(".el").join("runs").join(&run).join("record"))
        .unwrap_or_else(|e| panic!("no record file for {run}: {e}"));
    let record_json =
        el::core::text::json::parse(record.trim()).unwrap_or_else(|| panic!("record is not JSON"));
    assert_eq!(record_json.get("phase").and_then(|v| v.as_str()), Some("green"));
}

#[test]
fn green_refuses_a_changed_test() {
    let root = fresh("tdd-green-locked");
    let hello = crate_at_red(&root);
    let test_id = find_id(&hello, "answer_returns_42");
    let output = el_stdin(&hello, &["edit", &test_id, "--body", "-"], "assert_eq!(answer(), 44);");
    ok(&output, "edit test");

    let output = el(&hello, &["tdd", "green", "--json"]);
    assert!(!output.status.success(), "a changed test must not pass green");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EL_TEST_LOCKED"), "changed test must name EL_TEST_LOCKED: {stdout}");
}

#[test]
fn green_refuses_a_target_that_still_fails() {
    let root = fresh("tdd-green-failing");
    let hello = crate_at_red(&root);
    let answer_id = find_id(&hello, "answer");
    let output = el_stdin(&hello, &["edit", &answer_id, "--body", "-"], "44");
    ok(&output, "edit answer to 44");

    let output = el(&hello, &["tdd", "green", "--json"]);
    assert!(!output.status.success(), "a still-failing target must not pass green");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EL_GATE_FAILED"), "failing target must name EL_GATE_FAILED: {stdout}");
}
