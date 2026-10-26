// UC-11: editing one statement changes that statement and nothing else. The test
// proves it at the catalog level (ids and text) and at the source level (the lines
// above do not move), which are the two ways a statement editor can go wrong.

use crate::program::{self, el, el_stdin, ok};
use el::core::text::json::Json;

fn data(output: &std::process::Output) -> Json {
    let line = String::from_utf8_lossy(&output.stdout);
    el::core::text::json::parse(line.trim())
        .expect("our JSON parses")
        .get("data")
        .expect("data")
        .clone()
}

fn ids(data: &Json) -> Vec<String> {
    data.get("statements")
        .and_then(|v| v.as_array())
        .expect("ids")
        .iter()
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect()
}

fn texts(data: &Json) -> Vec<String> {
    data.get("statement_tree")
        .and_then(|v| v.as_array())
        .expect("tree")
        .iter()
        .filter_map(|stmt| stmt.get("text").and_then(|v| v.as_str()).map(str::to_string))
        .collect()
}

#[test]
fn editing_one_statement_keeps_every_other_statement_byte_identical() {
    let built = program::build("uc11");
    let demo = el_stdin(
        &built.hello,
        &["add", "fn", "demo", "--in", "src/greet.rs", "--sig", "fn demo()", "--body", "-"],
        "let a = 1;\nlet b = 2;\nlet c = 3;\n",
    );
    ok(&demo, "add demo");
    let demo = String::from_utf8_lossy(&demo.stdout).trim().to_string();
    let before = el(&built.hello, &["show", &demo, "--statements", "--json"]);
    ok(&before, "show before");
    let before = data(&before);
    let before_ids = ids(&before);
    let before_texts = texts(&before);
    assert_eq!(before_ids.len(), 3);
    let source_before = std::fs::read_to_string(built.hello.join("src/greet.rs")).expect("source");
    let line_before = source_before.lines().position(|line| line.contains("let a = 1;")).expect("a");

    let edit = el_stdin(
        &built.hello,
        &["edit", &before_ids[1], "--body", "-"],
        "let b = 20;\n",
    );
    ok(&edit, "edit statement");

    let after = el(&built.hello, &["show", &demo, "--statements", "--json"]);
    ok(&after, "show after");
    let after = data(&after);
    assert_eq!(ids(&after), before_ids);
    let after_texts = texts(&after);
    assert_eq!(after_texts[0], before_texts[0]);
    assert_eq!(after_texts[1], "let b = 20;");
    assert_eq!(after_texts[2], before_texts[2]);
    let source_after = std::fs::read_to_string(built.hello.join("src/greet.rs")).expect("source");
    assert_eq!(
        source_after.lines().position(|line| line.contains("let a = 1;")),
        Some(line_before)
    );
}
