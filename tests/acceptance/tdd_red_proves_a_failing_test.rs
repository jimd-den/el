// `el tdd red` proves the designated test fails by assertion, against
// unchanged production code and config, and freezes the test lock at
// red-time values for green to check against. A passing test, a test
// that cannot compile, or a timeout is not red — each is refused
// with `EL_INVALID_RED`, never counted as evidence.

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

fn crate_with_answer_and_test(
    root: &std::path::Path,
    test_body: &str,
    fast: bool,
) -> (std::path::PathBuf, String) {
    let hello = root.join("hello");
    ok(&el(root, &["new", "hello"]), "new");
    let output = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&output, "add answer");
    let mut args: Vec<&str> = vec!["add", "test", "answer_returns_42", "--in", "main", "--body", "-"];
    if fast {
        args.push("--fast");
    }
    let output = el_stdin(&hello, &args, test_body);
    ok(&output, "add test");
    let test_id = find_test_id(&hello, "answer_returns_42");
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "tdd start");
    (hello, test_id)
}

#[test]
fn red_accepts_an_assertion_failure_and_freezes_the_test_lock() {
    let root = fresh("tdd-red-accepts");
    let (hello, test_id) = crate_with_answer_and_test(&root, "assert_eq!(answer(), 43);", false);

    let output = el(&hello, &["tdd", "red", "--json"]);
    ok(&output, "tdd red");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("red"));
    let run = data.get("run").and_then(|v| v.as_str()).unwrap().to_string();
    let record = std::fs::read_to_string(hello.join(".el").join("runs").join(&run).join("record"))
        .unwrap_or_else(|e| panic!("no record file for {run}: {e}"));
    let record_json =
        el::core::text::json::parse(record.trim()).unwrap_or_else(|| panic!("record is not JSON"));
    assert_eq!(record_json.get("phase").and_then(|v| v.as_str()), Some("red"));
    let lock = record_json
        .get("test_lock")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("no test_lock: {record_json:?}"));
    assert!(
        lock.iter().any(|entry| entry.get("id").and_then(|v| v.as_str()) == Some(test_id.as_str())),
        "test lock must hold the target at red: {record_json:?}"
    );
}

#[test]
fn red_refuses_a_test_that_already_passes() {
    let root = fresh("tdd-red-passing");
    let (hello, _test_id) = crate_with_answer_and_test(&root, "assert_eq!(answer(), 42);", false);

    let output = el(&hello, &["tdd", "red", "--json"]);
    assert!(!output.status.success(), "a passing test is not red");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EL_INVALID_RED"), "passing red must name EL_INVALID_RED: {stdout}");
}

#[test]
fn red_refuses_a_test_that_cannot_compile() {
    let root = fresh("tdd-red-setup");
    let (hello, _test_id) =
        crate_with_answer_and_test(&root, "assert_eq!(answer(), nosuchfn());", true);

    let output = el(&hello, &["tdd", "red", "--json"]);
    assert!(!output.status.success(), "a setup error is not red");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EL_INVALID_RED"), "setup red must name EL_INVALID_RED: {stdout}");
}
