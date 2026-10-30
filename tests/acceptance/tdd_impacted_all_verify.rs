// `el tdd impacted` runs the tests calling production elements that
// changed since green, plus the locked target test; when a changed
// element has no name to scan for (an impl block), impact is
// unavailable and the full suite runs instead, recorded as
// `fallback: all`. `all` runs the canonical suite, `verify` proves
// the compiler passes and that nothing moved since the run's last
// blessed state, then closes the run. `abandon` closes a run early.

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

fn add_fn(hello: &std::path::Path, name: &str, sig: &str, body: &str) {
    let output = el_stdin(hello, &["add", "fn", name, "--in", "main", "--sig", sig, "--body", "-"], body);
    ok(&output, "add fn");
}

fn add_test(hello: &std::path::Path, name: &str, body: &str) {
    let output = el_stdin(hello, &["add", "test", name, "--in", "main", "--body", "-"], body);
    ok(&output, "add test");
}

/// A crate at green: answer fixed from 42 to 43, the target test
/// passing. With helper, a second passing test over a second fn.
fn crate_at_green(root: &std::path::Path, with_helper: bool) -> (std::path::PathBuf, String) {
    let hello = root.join("hello");
    ok(&el(root, &["new", "hello"]), "new");
    add_fn(&hello, "answer", "fn answer() -> u32", "42");
    add_fn(&hello, "main", "fn main()", "");
    add_test(&hello, "answer_returns_42", "assert_eq!(answer(), 43);");
    if with_helper {
        add_fn(&hello, "helper", "fn helper() -> u32", "1");
        add_test(&hello, "helper_returns_1", "assert_eq!(helper(), 1);");
    }
    let test_id = find_id(&hello, "answer_returns_42");
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "tdd start");
    ok(&el(&hello, &["tdd", "red"]), "tdd red");
    let answer_id = find_id(&hello, "answer");
    let output = el_stdin(&hello, &["edit", &answer_id, "--body", "-"], "43");
    ok(&output, "edit answer to 43");
    ok(&el(&hello, &["tdd", "green"]), "tdd green");
    (hello, test_id)
}

#[test]
fn impacted_runs_the_tests_calling_changed_production() {
    let root = fresh("tdd-impacted-named");
    let (hello, _test_id) = crate_at_green(&root, true);
    let helper_id = find_id(&hello, "helper");
    let output = el_stdin(&hello, &["edit", &helper_id, "--body", "-"], "1 + 0");
    ok(&output, "edit helper");

    let output = el(&hello, &["tdd", "impacted", "--json"]);
    ok(&output, "tdd impacted");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("impacted"));
    assert_ne!(data.get("fallback").and_then(|v| v.as_str()), Some("all"));
}

#[test]
fn strict_run_falls_back_to_all_when_impact_is_unavailable() {
    let root = fresh("tdd-impacted-fallback");
    let (hello, _test_id) = crate_at_green(&root, false);
    let output = el_stdin(
        &hello,
        &["add", "struct", "Point", "--in", "main", "--sig", "struct Point", "--body", "-"],
        "x: u32",
    );
    ok(&output, "add struct");
    let output = el_stdin(
        &hello,
        &["add", "impl", "Point", "--in", "main", "--sig", "impl Point", "--body", "-"],
        "fn get(&self) -> u32 {\n  self.x\n}",
    );
    ok(&output, "add impl");
    // The struct and impl were added after green, so both are
    // unblessed production changes; the impl has no name to scan
    // for, so impacted must fall back to the full suite.
    let output = el(&hello, &["tdd", "impacted", "--json"]);
    ok(&output, "tdd impacted");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("impacted"));
    assert_eq!(data.get("fallback").and_then(|v| v.as_str()), Some("all"));
}

#[test]
fn all_runs_the_full_suite_after_impacted() {
    let root = fresh("tdd-all");
    let (hello, _test_id) = crate_at_green(&root, false);
    ok(&el(&hello, &["tdd", "impacted"]), "tdd impacted");

    let output = el(&hello, &["tdd", "all", "--json"]);
    ok(&output, "tdd all");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("all"));
    let evidence = json.get("evidence").unwrap_or_else(|| panic!("no evidence: {json:?}"));
    let command = evidence.get("command").and_then(|v| v.as_array()).unwrap();
    assert_eq!(command.len(), 2, "all runs cargo test with no filter: {command:?}");
}

#[test]
fn verify_proves_check_and_closes_the_run() {
    let root = fresh("tdd-verify");
    let (hello, test_id) = crate_at_green(&root, false);
    ok(&el(&hello, &["tdd", "impacted"]), "tdd impacted");
    ok(&el(&hello, &["tdd", "all"]), "tdd all");

    let output = el(&hello, &["tdd", "verify", "--json"]);
    ok(&output, "tdd verify");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("phase").and_then(|v| v.as_str()), Some("verify"));
    assert!(
        std::fs::read_to_string(hello.join(".el").join("runs").join("active")).is_err(),
        "verify must close the run"
    );
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "a new start works after verify");
}

#[test]
fn abandon_closes_a_run_early() {
    let root = fresh("tdd-abandon");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_fn(&hello, "answer", "fn answer() -> u32", "42");
    add_test(&hello, "answer_returns_42", "assert_eq!(answer(), 43);");
    let test_id = find_id(&hello, "answer_returns_42");
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "tdd start");

    let output = el(&hello, &["tdd", "abandon", "--json"]);
    ok(&output, "tdd abandon");
    assert!(
        std::fs::read_to_string(hello.join(".el").join("runs").join("active")).is_err(),
        "abandon must clear the active marker"
    );
    ok(&el(&hello, &["tdd", "start", "--target", &test_id]), "a new start works after abandon");
}
