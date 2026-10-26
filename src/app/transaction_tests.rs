// The transaction is the part a user cannot inspect after the fact, so its
// boundaries are tested as data: the head, the journal length, the bytes on the
// fake disk, and the inflight marker. Every test states what must *not* have
// happened, which is the half a happy-path test never sees.

use super::support::*;
use crate::app::Exit;
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::change::{Anchor, Change};
use crate::core::model::id::ElId;
use crate::ports::fs_fake::Memory;
use crate::ports::Fs;
use crate::adapters::store::Store;

#[test]
fn a_model_violation_writes_nothing() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let writes = memory.writes().len();
    let mut mutation = add();
    mutation.changes = vec![Change::AddStmt {
        owner: ElId(1),
        body: Body {
            statements: vec![BodyStmt {
                text: "unsafe { poke(); }".to_string(),
                children: Vec::new(),
            }],
        },
        anchor: Anchor::End,
    }];

    let report = run(&memory, &store, &Fixed(1), &Lint(true), mutation);

    assert_eq!(report.exit(), Exit::Violation);
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(store.entries().expect("entries").len(), 1);
    assert_eq!(memory.writes().len(), writes, "a refusal wrote a file");
}

#[test]
fn a_dry_run_restores_the_tree_and_records_preview() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    let mut mutation = add();
    mutation.dry_run = true;

    let report = run(&memory, &store, &Fixed(1), &Lint(true), mutation);

    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.gate.expect("gate").verdict, "preview");
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    assert_eq!(store.entry(2).expect("entry").expect("present").gate, "preview:0");
}

#[test]
fn a_strict_lint_refusal_restores_the_tree_and_keeps_head() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    let mut mutation = add();
    mutation.strict = true;

    let report = run(&memory, &store, &Fixed(1), &Lint(false), mutation);

    assert_eq!(report.error.expect("refused").code, "gate.rejected");
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    assert_eq!(store.entry(2).expect("entry").expect("present").gate, "rejected:1");
    assert_eq!(store.inflight().expect("inflight"), None);
}
