// A rejection that lists four errors has told the agent four things and asked it
// to do four things at once. Exactly one is marked `first_actionable`, so the loop
// has one next move; the mark is deterministic, so two identical attempts do not
// disagree about where to start.

use crate::program::{el, el_stdin, fresh, ok};

const BATCH: &str = r#"[
  {"kind":"fn","name":"one","sig":"fn one() -> u32","body":"\"one\""},
  {"kind":"fn","name":"two","sig":"fn two() -> u32","body":"\"two\""},
  {"kind":"fn","name":"three","sig":"fn three() -> u32","body":"\"three\""},
  {"kind":"fn","name":"four","sig":"fn four() -> u32","body":"\"four\""}
]"#;

fn attempt(name: &str) -> el::core::text::json::Json {
    let root = fresh(name);
    let hello = root.join("hello");
    ok(&el(&root, &["new", "hello"]), "new");

    let output = el_stdin(&hello, &["batch", "--json"], BATCH);

    assert_eq!(
        output.status.code(),
        Some(3),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(text.trim()).expect("one JSON object")
}

fn diagnostics(json: &el::core::text::json::Json) -> Vec<el::core::text::json::Json> {
    json.get("gate")
        .and_then(|gate| gate.get("diagnostics"))
        .and_then(|list| list.as_array())
        .expect("diagnostics")
        .to_vec()
}

#[test]
fn four_new_errors_report_four_and_mark_exactly_one() {
    let json = attempt("first-actionable-four");
    let list = diagnostics(&json);

    assert_eq!(list.len(), 4, "each error is reported");
    let marked: Vec<&el::core::text::json::Json> = list
        .iter()
        .filter(|diag| matches!(diag.get("first_actionable"), Some(el::core::text::json::Json::Bool(true))))
        .collect();
    assert_eq!(marked.len(), 1, "exactly one is first actionable");
}

#[test]
fn the_mark_is_deterministic() {
    let first = attempt("first-actionable-a");
    let second = attempt("first-actionable-b");

    let mark = |json: &el::core::text::json::Json| {
        diagnostics(json)
            .into_iter()
            .find(|diag| matches!(diag.get("first_actionable"), Some(el::core::text::json::Json::Bool(true))))
            .map(|diag| {
                (
                    diag.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    diag.get("el").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                )
            })
    };

    assert_eq!(mark(&first), mark(&second));
}
