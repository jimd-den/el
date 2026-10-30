// `el read` is revision-bound: a `--rev` pin from an earlier read
// that no longer matches the target is `EL_STALE_REF`, with the
// expected and observed digests named — never a silent stale answer.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn add_answer(hello: &std::path::Path) {
    let output = el_stdin(
        hello,
        &["add", "fn", "answer", "--in", "main", "--sig", "fn answer() -> u32", "--body", "-"],
        "42",
    );
    ok(&output, "add answer");
}

#[test]
fn read_with_old_digest_returns_el_stale_ref() {
    let root = fresh("read-stale-ref");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_answer(&hello);

    let first = el(&hello, &["read", "main::fn/answer", "--json"]);
    ok(&first, "read");
    let json = json_of(&first);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    let rev = data.get("rev").and_then(|v| v.as_str()).unwrap().to_string();
    assert!(rev.starts_with("sha256:"), "rev: {rev}");
    let id = data.get("id").and_then(|v| v.as_str()).unwrap().to_string();
    let lines = data.get("lines").and_then(|v| v.as_array()).unwrap();
    assert!(lines.iter().any(|line| line.as_str().unwrap_or("").contains("fn answer")));

    let narrow = el(&hello, &["read", "main::fn/answer", "--lines", "1:1", "--json"]);
    ok(&narrow, "read --lines");
    let json = json_of(&narrow);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.get("lines").and_then(|v| v.as_array()).map(|a| a.len()), Some(1));
    assert!(matches!(data.get("truncated"), Some(Json::Bool(true))));

    let edited = el_stdin(
        &hello,
        &["edit", &id, "--body", "-"],
        "43",
    );
    ok(&edited, "edit answer");

    let stale = el(&hello, &["read", "main::fn/answer", "--rev", &rev, "--json"]);
    assert_eq!(stale.status.code(), Some(5), "stale exit");
    let json = json_of(&stale);
    assert!(matches!(json.get("ok"), Some(Json::Bool(false))));
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("blocked"));
    let diagnostics = json.get("diagnostics").and_then(|v| v.as_array()).unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].get("code").and_then(|v| v.as_str()), Some("EL_STALE_REF"));
    assert_eq!(diagnostics[0].get("expected").and_then(|v| v.as_str()), Some(rev.as_str()));
    let observed = diagnostics[0].get("observed").and_then(|v| v.as_str()).unwrap();
    assert!(observed.starts_with("sha256:") && observed != rev);

    let fresh = el(&hello, &["read", "main::fn/answer", "--rev", observed, "--json"]);
    ok(&fresh, "read with fresh rev");
    let json = json_of(&fresh);
    assert_eq!(
        json.get("data").and_then(|v| v.get("rev")).and_then(|v| v.as_str()),
        Some(observed)
    );
}
