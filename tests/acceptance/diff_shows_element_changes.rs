// FR-32: diff names the statement, not a line range. The JSON change list carries
// the id and the old/new lines; the human view says the same id.

use crate::program::{self, el, el_stdin, ok};

#[test]
fn diff_names_the_statement_that_changed() {
    let built = program::build("diff");
    let demo = el_stdin(
        &built.hello,
        &["add", "fn", "demo", "--in", "src/greet.rs", "--sig", "fn demo()", "--body", "-"],
        "let a = 1;\nlet b = 2;\nlet c = 3;\n",
    );
    ok(&demo, "add demo");
    let demo = String::from_utf8_lossy(&demo.stdout).trim().to_string();
    let show = el(&built.hello, &["show", &demo, "--statements", "--json"]);
    ok(&show, "show");
    let line = String::from_utf8_lossy(&show.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("json");
    let ids = parsed.get("data").and_then(|d| d.get("statements")).and_then(|v| v.as_array()).expect("ids");
    let second = ids[1].as_str().expect("stmt id").to_string();
    ok(
        &el_stdin(&built.hello, &["edit", &second, "--body", "-"], "let b = 20;\n"),
        "edit",
    );

    let json = el(&built.hello, &["diff", "--json"]);
    ok(&json, "diff --json");
    let line = String::from_utf8_lossy(&json.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("json");
    let elements = parsed
        .get("data")
        .and_then(|d| d.get("elements"))
        .and_then(|v| v.as_array())
        .expect("elements");
    assert!(elements.iter().any(|change| {
        change.get("id").and_then(|v| v.as_str()) == Some(second.as_str())
            && change.get("op").and_then(|v| v.as_str()) == Some("changed")
            && line.contains("let b = 20;")
    }), "{line}");

    let human = el(&built.hello, &["diff"]);
    ok(&human, "diff");
    assert!(String::from_utf8_lossy(&human.stdout).contains(&second));
}
