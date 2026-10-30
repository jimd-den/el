// The versioned JSON envelope: every `--json` response, success or
// refusal, carries the same required top-level fields, so a script
// never has to guess which shape a verb returns.

use crate::program::{el, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn required_fields(json: &Json) {
    for key in [
        "schema_version",
        "request_id",
        "ok",
        "action",
        "status",
        "data",
        "refs",
        "diagnostics",
        "next",
    ] {
        assert!(json.get(key).is_some(), "envelope lacks {key}: {json:?}");
    }
}

#[test]
fn a_successful_response_carries_the_envelope() {
    let root = fresh("envelope-ok");
    let output = el(&root, &["version", "--json"]);

    ok(&output, "version --json");
    let json = json_of(&output);
    required_fields(&json);
    assert_eq!(json.get("schema_version").and_then(|v| v.as_str()), Some("1"));
    assert_eq!(json.get("action").and_then(|v| v.as_str()), Some("version"));
    assert!(matches!(json.get("ok"), Some(Json::Bool(true))));
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("passed"));
    assert_eq!(json.get("refs").and_then(|v| v.as_array()).map(|a| a.len()), Some(0));
    assert_eq!(
        json.get("diagnostics").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(0)
    );
    assert_eq!(json.get("next").and_then(|v| v.as_array()).map(|a| a.len()), Some(0));
    let request_id = json.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
    assert!(request_id.starts_with("req-"), "request_id: {request_id}");
}

#[test]
fn a_refusal_carries_the_envelope_and_a_typed_diagnostic() {
    let root = fresh("envelope-refused");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let output = el(&hello, &["show", "el-9999", "--json"]);

    assert_eq!(output.status.code(), Some(6));
    let json = json_of(&output);
    required_fields(&json);
    assert!(matches!(json.get("ok"), Some(Json::Bool(false))));
    assert_eq!(json.get("action").and_then(|v| v.as_str()), Some("show"));
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("refused"));
    let diagnostics = json.get("diagnostics").and_then(|v| v.as_array()).unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].get("code").and_then(|v| v.as_str()),
        Some("notfound.element")
    );
    let next = json.get("next").and_then(|v| v.as_array()).unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].as_str(), Some("el ls"));
}
