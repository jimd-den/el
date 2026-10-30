// `el discover` returns bounded, ranked results with a cursor —
// never a full-file dump, and never a silent truncation.

use crate::program::{el, el_stdin, fresh, ok};
use el::core::text::json::Json;

fn json_of(output: &std::process::Output) -> Json {
    let stdout = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(stdout.trim())
        .unwrap_or_else(|| panic!("not one JSON object: {stdout}"))
}

fn add_fn(hello: &std::path::Path, name: &str) {
    let output = el_stdin(
        hello,
        &["add", "fn", name, "--in", "main", "--sig", &format!("fn {name}() -> u32"), "--body", "-"],
        "42",
    );
    ok(&output, "add fn");
}

#[test]
fn discover_pages_bounded_results_with_a_cursor() {
    let root = fresh("discover-bounded");
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");
    add_fn(&hello, "answer");
    add_fn(&hello, "answer_plus");
    add_fn(&hello, "answer_check");
    add_fn(&hello, "unrelated");

    let first = el(&hello, &["discover", "--query", "answer", "--limit", "2", "--json"]);
    ok(&first, "discover page 1");
    let json = json_of(&first);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    let rows = data.as_array().unwrap();
    assert_eq!(rows.len(), 2, "limit bounds the page: {json:?}");
    let pagination = json.get("pagination").unwrap_or_else(|| panic!("no pagination: {json:?}"));
    assert!(matches!(pagination.get("truncated"), Some(Json::Bool(true))));
    assert_eq!(pagination.get("next_cursor").and_then(|v| v.as_number()), Some(2));
    let refs = json.get("refs").and_then(|v| v.as_array()).unwrap();
    assert_eq!(refs.len(), 2, "refs stand on the returned page: {json:?}");
    assert!(refs[0].get("rev").and_then(|v| v.as_str()).unwrap_or("").starts_with("sha256:"));

    let second = el(&hello, &["discover", "--query", "answer", "--limit", "2", "--cursor", "2", "--json"]);
    ok(&second, "discover page 2");
    let json = json_of(&second);
    let data = json.get("data").unwrap_or_else(|| panic!("no data: {json:?}"));
    assert_eq!(data.as_array().unwrap().len(), 1, "last page: {json:?}");
    let pagination = json.get("pagination").unwrap_or_else(|| panic!("no pagination: {json:?}"));
    assert!(matches!(pagination.get("truncated"), Some(Json::Bool(false))));
    assert!(matches!(pagination.get("next_cursor"), Some(Json::Null)));
}
