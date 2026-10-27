// `el explain` answers why an element exists without opening its file: the
// module's preamble, the doc text, the callers, the tests that name it, and the
// history that touched it. Catalog-only, and it says so, because rust-analyzer is
// not built yet.

use crate::program::{build, el, el_stdin, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn explain_names_the_preamble_doc_callers_tests_and_history() {
    let built = build("explain-surface");
    let added = el_stdin(
        &built.hello,
        &["add", "fn", "test_greet", "--sig", "fn test_greet()", "--body", "-"],
        "greet::greet(\"test\");\n",
    );
    ok(&added, "add test_greet");

    let output = el(&built.hello, &["explain", &built.greet]);

    ok(&output, "explain");
    let shown = text(&output);
    assert!(shown.contains("greet::fn/greet"), "{shown}");
    assert!(shown.contains("callers"), "{shown}");
    assert!(shown.contains(&built.main), "the caller's id is named:\n{shown}");
    assert!(shown.contains("test_greet"), "the test is named:\n{shown}");
    assert!(shown.contains("history"), "{shown}");
    assert!(shown.contains("catalog"), "the source is named:\n{shown}");
}

#[test]
fn explain_json_carries_the_same_sections() {
    let built = build("explain-json");

    let output = el(&built.hello, &["explain", &built.greet, "--json"]);

    ok(&output, "explain --json");
    let text = String::from_utf8_lossy(&output.stdout);
    let json = el::core::text::json::parse(text.trim()).expect("one object");
    let data = json.get("data").expect("data");
    assert!(data.get("callers").and_then(|v| v.as_array()).is_some(), "{text}");
    assert!(data.get("tests").and_then(|v| v.as_array()).is_some(), "{text}");
    assert!(data.get("history").and_then(|v| v.as_array()).is_some(), "{text}");
    assert_eq!(
        data.get("source").and_then(|v| v.as_str()),
        Some("catalog"),
        "{text}"
    );
}
