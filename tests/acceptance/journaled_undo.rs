// ADR-0032: undo and redo append journal entries instead of silently
// moving head, so every head state is a journaled sequence and log
// alone orients the agent.

use crate::program::{self, el, el_stdin, files, log_json, ok};

fn text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn seq_of(entry: &el::core::text::json::Json) -> u64 {
    entry
        .get("seq")
        .and_then(|v| v.as_number())
        .expect("seq")
}

fn add_tmp_fn(hello: &std::path::Path) -> String {
    ok(&el(hello, &["add", "mod", "src/tmp.rs"]), "add mod");
    let added = el_stdin(
        hello,
        &[
            "add", "fn", "tmp", "--in", "src/tmp.rs", "--sig", "fn tmp()",
            "--body", "-", "--fast",
        ],
        "go();\n",
    );
    ok(&added, "add fn");
    String::from_utf8_lossy(&added.stdout).trim().to_string()
}

#[test]
fn undo_appends_an_entry_naming_what_it_undid() {
    let built = program::build("journaled-undo");
    let function = add_tmp_fn(&built.hello);
    let tip = seq_of(&log_json(&built.hello, false)[0]);

    let undo = el(&built.hello, &["undo"]);
    ok(&undo, "undo");
    let out = text(&undo);
    assert!(out.contains("undid"), "undo names what it undid: {out}");
    assert!(out.contains("el redo"), "undo names the road back: {out}");

    let after = log_json(&built.hello, false);
    assert_eq!(
        after[0].get("op").and_then(|v| v.as_str()),
        Some("undo"),
        "the newest entry is the undo"
    );
    assert!(
        after[0]
            .get("msg")
            .and_then(|v| v.as_str())
            .expect("msg")
            .contains("add fn"),
        "the undo entry names the undone change"
    );
    assert_eq!(seq_of(&after[0]), tip + 1, "head advanced by one entry");

    let bytes = files(&built.hello);
    let tmp = bytes
        .get("src/tmp.rs")
        .expect("tmp.rs still exists as a module file");
    assert!(
        !String::from_utf8_lossy(tmp).contains("fn tmp"),
        "the undone fn is gone from the tree"
    );
    let _ = function;
}

#[test]
fn redo_after_journaled_undo_restores_the_undone_state() {
    let built = program::build("journaled-redo");
    add_tmp_fn(&built.hello);
    let before = files(&built.hello);
    ok(&el(&built.hello, &["undo"]), "undo");

    let redo = el(&built.hello, &["redo"]);
    ok(&redo, "redo");

    assert_eq!(files(&built.hello), before, "redo restores the bytes");
    let after = log_json(&built.hello, false);
    assert_eq!(
        after[0].get("op").and_then(|v| v.as_str()),
        Some("redo"),
        "the newest entry is the redo"
    );
}

#[test]
fn undo_two_steps_then_two_redos_restore() {
    let built = program::build("journaled-redo-steps");
    let before = files(&built.hello);
    ok(&el(&built.hello, &["undo", "2"]), "undo 2");
    assert_ne!(files(&built.hello), before, "undo moved the tree");

    ok(&el(&built.hello, &["redo"]), "redo");
    ok(&el(&built.hello, &["redo"]), "redo again");

    assert_eq!(files(&built.hello), before, "two redos restore the bytes");
}
