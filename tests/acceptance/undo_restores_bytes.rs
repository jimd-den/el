// UC-3 / FR-29: undo is a full materialize, including deletions.
// ADR-0032: undo appends one entry per step back, so head after
// undo 4 is tip + 4, not the restored sequence. The bytes
// assertions are unchanged; only the head arithmetic moved.

use crate::program::{self, el, el_stdin, files, log_json, ok};

#[test]
fn undo_restores_every_owned_byte_and_the_deleted_file() {
    let built = program::build("undo");
    let before = files(&built.hello);

    let module = el(&built.hello, &["add", "mod", "src/tmp.rs"]);
    ok(&module, "add mod");
    let module = String::from_utf8_lossy(&module.stdout).trim().to_string();
    let added = el_stdin(
        &built.hello,
        &["add", "fn", "tmp", "--in", "src/tmp.rs", "--sig", "fn tmp()", "--body", "-", "--fast"],
        "go();\n",
    );
    ok(&added, "add fn");
    let function = String::from_utf8_lossy(&added.stdout).trim().to_string();
    ok(&el(&built.hello, &["rm", &function]), "rm fn");
    ok(&el(&built.hello, &["rm", &module]), "rm mod");
    assert!(!built.hello.join("src/tmp.rs").exists(), "the deletion is real");

    let tip = log_json(&built.hello, false)[0]
        .get("seq")
        .and_then(|v| v.as_number())
        .expect("tip");

    let undo = el(&built.hello, &["undo", "4"]);
    ok(&undo, "undo 4");

    assert_eq!(files(&built.hello), before);
    let head: u64 = std::fs::read_to_string(built.hello.join(".el/head"))
        .expect("head")
        .trim()
        .parse()
        .expect("number");
    assert_eq!(head, tip + 4);
}

#[test]
fn redo_restores_the_same_bytes_after_undo() {
    let built = program::build("redo");
    let before = files(&built.hello);
    ok(&el(&built.hello, &["undo", "2"]), "undo");
    assert_ne!(files(&built.hello), before);

    ok(&el(&built.hello, &["redo"]), "redo");
    ok(&el(&built.hello, &["redo"]), "redo again");

    assert_eq!(files(&built.hello), before);
}
