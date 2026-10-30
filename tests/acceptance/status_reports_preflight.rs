// `el status` is the preflight: root, crate, catalog presence, journal
// head, policy, and tool availability — never a claim that tests passed
// merely because they have not run.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

#[test]
fn status_reports_root_head_policy_and_tools() {
    let root = fresh("status-preflight");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    let add = el_stdin(
        &hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&add, "add answer");

    let output = el(&hello, &["status", "--json"]);
    ok(&output, "status --json");
    let json = json_of(&output);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("root").and_then(|v| v.as_str()), Some("."));
    assert_eq!(data.get("crate").and_then(|v| v.as_str()), Some("hello"));
    assert!(matches!(data.get("catalog"), Some(Json::Bool(true))));
    assert!(data.get("head_seq").and_then(|v| v.as_number()).is_some());
    let policy = data.get("policy").unwrap_or_else(|| panic!("no policy: {json:?}"));
    assert_eq!(policy.get("gate").and_then(|v| v.as_str()), Some("check"));
    assert!(policy.get("split").and_then(|v| v.as_str()).is_some());
    assert_eq!(data.get("cargo").and_then(|v| v.as_str()), Some("present"));
    assert!(matches!(data.get("active_run"), Some(Json::Null)));
    assert!(matches!(data.get("test"), Some(Json::Str(_))));
    assert_eq!(data.get("test").and_then(|v| v.as_str()), Some("not_run"));

    let human = el(&hello, &["status"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("head"), "human status:\n{text}");
}
