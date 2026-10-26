// Recovery and drift are the two places a user's hand touches the tree. Both are
// about not trusting the disk on the way in: a marker means the last transaction
// did not finish, and a differing byte means the catalog and the file disagree.

use super::support::*;
use super::*;
use crate::app::Env;
use crate::ports::fs_fake::Memory;
use crate::ports::Fs;
use crate::adapters::store::Store;

#[test]
fn recovery_restores_head_before_the_next_command() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    memory.write("src/main.rs", b"half-written").expect("drift");
    store.set_inflight("2 deadbeef 1").expect("inflight");
    let env = Env {
        fs: &memory,
        journal: &store,
        clock: &Fixed(1),
        tools: &Lint(true),
        actor: "human".to_string(),
        crate_name: "demo".to_string(),
        root: ".".to_string(),
    };

    let notes = recover(&env).expect("recovered");

    assert_eq!(notes.len(), 1);
    assert_eq!(store.inflight().expect("inflight"), None);
    assert!(
        String::from_utf8(memory.read("src/main.rs").expect("read").expect("file"))
            .expect("text")
            .contains("fn run"),
    );
}

#[test]
fn drift_before_a_change_is_refused_without_writing() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    memory
        .write("src/main.rs", b"fn run() { changed_by_hand(); }\n")
        .expect("drift");
    let writes = memory.writes().len();

    let report = run(&memory, &store, &Fixed(1), &Lint(true), add());

    assert_eq!(report.error.expect("drift").code, "env.drift");
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(store.entries().expect("entries").len(), 1);
    assert_eq!(memory.writes().len(), writes);
}
