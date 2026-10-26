// UC-12: insert one statement into a body without reading the body in full.
// The anchor decides the position; the ids of every existing statement survive,
// and a cross-body anchor is an L0 refusal rather than a guess.

use crate::program::{self, el, el_stdin, ok};
use el::core::text::json::Json;

fn show(hello: &std::path::Path, id: &str) -> Json {
    let output = el(hello, &["show", id, "--statements", "--json"]);
    ok(&output, "show");
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

fn one(hello: &std::path::Path, owner: &str, body: &str, anchor: &str, anchor_id: &str) -> String {
    let output = el_stdin(
        hello,
        &["add", "stmt", owner, anchor, anchor_id, "--body", "-"],
        body,
    );
    ok(&output, "add stmt");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn a_statement_inserts_after_its_anchor_and_reorders_without_losing_its_id() {
    let built = program::build("uc12");
    let demo = el_stdin(
        &built.hello,
        &["add", "fn", "demo", "--in", "src/greet.rs", "--sig", "fn demo()", "--body", "-"],
        "let a = 1;\nlet b = 2;\n",
    );
    ok(&demo, "add demo");
    let demo = String::from_utf8_lossy(&demo.stdout).trim().to_string();
    let before = ids(&show(&built.hello, &demo));
    assert_eq!(before.len(), 2);

    let inserted = one(&built.hello, &demo, "debug_assert!(true);", "--after", &before[0]);

    let after = ids(&show(&built.hello, &demo));
    assert_eq!(after, vec![before[0].clone(), inserted.clone(), before[1].clone()]);

    let reorder = el(&built.hello, &["mv", &inserted, "--after", &before[1]]);
    ok(&reorder, "reorder");
    let after = ids(&show(&built.hello, &demo));
    assert_eq!(after, vec![before[0].clone(), before[1].clone(), inserted.clone()]);
}

#[test]
fn an_anchor_in_another_body_is_refused_before_it_is_written() {
    let built = program::build("uc12-cross");
    let first = el_stdin(
        &built.hello,
        &["add", "fn", "first", "--in", "src/greet.rs", "--sig", "fn first()", "--body", "-"],
        "let a = 1;\n",
    );
    ok(&first, "first");
    let second = el_stdin(
        &built.hello,
        &["add", "fn", "second", "--in", "src/greet.rs", "--sig", "fn second()", "--body", "-"],
        "let b = 2;\n",
    );
    ok(&second, "second");
    let second = String::from_utf8_lossy(&second.stdout).trim().to_string();
    let anchor = ids(&show(&built.hello, &first_id(&first)))[0].clone();

    let refused = el_stdin(
        &built.hello,
        &["add", "stmt", &second, "--after", &anchor, "--body", "-"],
        "let c = 3;\n",
    );

    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stdout).contains("model.cross_body_move"));
}

fn first_id(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
