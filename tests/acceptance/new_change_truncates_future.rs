// ADR-0032 supersedes FR-30's shape: undo appends entries, so a new
// change after undo orphans nothing to the attic — the undone entries
// stay in the journal, revertible by sequence. What survives from
// FR-30 is the redo rule: a new mutation after undos still ends
// redo, because the trailing undo/redo run is broken.

use crate::program::{self, el, el_stdin, files, log_json, ok};

fn number(value: Option<&el::core::text::json::Json>) -> u64 {
    value.and_then(|v| v.as_number()).expect("number")
}

fn ops(entries: &[el::core::text::json::Json]) -> Vec<String> {
    entries
        .iter()
        .map(|e| {
            e.get("op")
                .and_then(|v| v.as_str())
                .expect("op")
                .to_string()
        })
        .collect()
}

#[test]
fn a_new_change_after_undo_keeps_the_undone_history_journaled() {
    let built = program::build("attic");
    let before = files(&built.hello);
    let tip = number(log_json(&built.hello, false)[0].get("seq"));

    ok(&el(&built.hello, &["undo", "2"]), "undo 2");

    let added = el_stdin(
        &built.hello,
        &["add", "fn", "future", "--in", "src/greet.rs", "--sig", "fn future()", "--body", "-", "--fast"],
        "go();\n",
    );
    ok(&added, "add after undo");

    let journal = log_json(&built.hello, false);
    let attic = log_json(&built.hello, true);
    assert_eq!(
        attic.len(),
        journal.len(),
        "nothing was orphaned to the attic by this undo plus change"
    );
    let journal_ops = ops(&journal);
    assert!(
        journal_ops.contains(&"undo".to_string()),
        "the undo entries stay journaled: {journal_ops:?}"
    );
    assert!(
        journal.iter().any(|e| number(e.get("seq")) == tip),
        "the undone tip sequence stays journaled"
    );

    let redo = el(&built.hello, &["redo"]);
    assert_eq!(redo.status.code(), Some(6), "redo ends at a new change");

    ok(&el(&built.hello, &["revert", &tip.to_string()]), "revert");
    assert_eq!(
        files(&built.hello),
        before,
        "the undone state stays reachable by revert"
    );
}
