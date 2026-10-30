// A missing cargo is an environment failure (`EL_ENV_UNAVAILABLE`,
// exit 4, status `blocked`) — never a gate failure, and never a claim
// about the code.

use crate::program::{el, el_env, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

#[test]
fn check_without_cargo_is_env_unavailable_not_gate_failed() {
    let root = fresh("missing-cargo-check");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let added = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&added, "add answer");

    let output = el_env(&hello, &["check", "--json"], "/nonexistent");
    assert_eq!(output.status.code(), Some(4));
    let json = json_of(&output);
    assert!(matches!(json.get("ok"), Some(Json::Bool(false))));
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("blocked"));
    let diagnostics = json.get("diagnostics").and_then(|v| v.as_array()).unwrap();
    assert_eq!(diagnostics[0].get("code").and_then(|v| v.as_str()), Some("EL_ENV_UNAVAILABLE"));
}
