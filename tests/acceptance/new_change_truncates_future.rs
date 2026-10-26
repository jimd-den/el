// FR-30: undo leaves the future redoable until a new mutation; then the future
// moves to attic and is still readable, while redo is gone.

use crate::program::{self, el, el_stdin, log_json, ok};

#[test]
fn a_new_change_moves_the_undone_future_to_the_attic() {
    let built = program::build("attic");
    ok(&el(&built.hello, &["undo", "2"]), "undo 2");
    let future = log_json(&built.hello, true)
        .first()
        .and_then(|entry| entry.get("seq"))
        .and_then(|seq| seq.as_number())
        .expect("future seq");

    let added = el_stdin(
        &built.hello,
        &["add", "fn", "future", "--in", "src/greet.rs", "--sig", "fn future()", "--body", "-"],
        "go();\n",
    );
    ok(&added, "add after undo");

    let journal = log_json(&built.hello, false);
    let attic = log_json(&built.hello, true);
    assert_eq!(journal.len(), 4);
    assert!(attic.len() > journal.len());
    assert!(
        attic
            .iter()
            .any(|entry| entry.get("seq").and_then(|s| s.as_number()) == Some(future))
    );
    let redo = el(&built.hello, &["redo"]);
    assert_eq!(redo.status.code(), Some(6));
}
