// M7's read surface, tested where it matters: an agent can learn every element id
// and one full body from `el tree` + `el show` without opening a `.rs` file. The
// test itself never reads one; the only non-source file it touches is the catalog,
// to know which ids the tree must contain.

use crate::program::{self, el, ok};
use std::fs;

#[test]
fn tree_and_show_navigate_the_whole_crate_without_source_files() {
    let built = program::build("navigate");
    let ids = program::element_ids(&built.hello);

    let tree = el(&built.hello, &["tree"]);
    ok(&tree, "el tree");
    let tree = String::from_utf8_lossy(&tree.stdout);
    for id in &ids {
        assert!(tree.contains(id), "tree is missing {id}:\n{tree}");
    }
    assert!(!tree.contains(r#"format!("hello, {name}")"#), "tree read a body without --statements");

    let detailed = el(&built.hello, &["tree", "--statements"]);
    ok(&detailed, "el tree --statements");
    let detailed = String::from_utf8_lossy(&detailed.stdout);
    assert!(detailed.contains(r#"format!("hello, {name}")"#), "{detailed}");
    assert!(detailed.contains("/// Greets by name."), "{detailed}");
    assert!(detailed.contains("greet::greet(\"world\");"), "{detailed}");

    let show = el(
        &built.hello,
        &["show", &built.greet, "--statements", "--source", "--history"],
    );
    ok(&show, "el show");
    let show = String::from_utf8_lossy(&show.stdout);
    for field in [
        "id", "kind", "alias", "module", "visibility", "signature", "doc", "mode",
        "sequence", "statements", "source", "history",
    ] {
        assert!(show.contains(field), "show is missing {field}:\n{show}");
    }
    assert!(show.contains(r#"format!("hello, {name}")"#), "{show}");
    assert!(show.contains("/// Greets by name."), "{show}");
    assert!(show.contains(&built.greet), "{show}");
}

#[test]
fn the_read_surface_answers_when_the_source_files_are_gone() {
    let built = program::build("no-source");
    fs::remove_file(built.hello.join("src/greet.rs")).expect("remove greet");
    fs::remove_file(built.hello.join("src/main.rs")).expect("remove main");

    let tree = el(&built.hello, &["tree", "--statements"]);
    ok(&tree, "el tree without source");
    let show = el(&built.hello, &["show", &built.greet, "--source"]);
    ok(&show, "el show without source");

    assert!(String::from_utf8_lossy(&show.stdout).contains("format!(\"hello, {name}\")"));
}

#[test]
fn tree_json_round_trips_through_the_project_reader() {
    let built = program::build("tree-json");

    let output = el(&built.hello, &["tree", "--statements", "--json"]);
    ok(&output, "el tree --json");
    let line = String::from_utf8_lossy(&output.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("our own JSON parses");

    assert_eq!(parsed.get("cmd").and_then(|v| v.as_str()), Some("tree"));
    assert!(line.contains(r#""ok":true"#), "{line}");
    let data = parsed.get("data").expect("tree payload");
    let modules = data.get("modules").and_then(|v| v.as_array()).expect("modules");
    let item = modules[0]
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .expect("an item");
    assert_eq!(item.get("id").and_then(|v| v.as_str()), Some(built.greet.as_str()));
    let statements = item
        .get("statement_tree")
        .and_then(|v| v.as_array())
        .expect("statements");
    assert!(
        statements.iter().any(|statement| statement
            .get("text")
            .and_then(|v| v.as_str())
            .is_some_and(|text| text.contains("format!(\"hello, {name}\")"))),
        "{line}"
    );
}
