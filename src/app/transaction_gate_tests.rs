// The compiler gate at the transaction boundary: what a mutation leaves
// behind when the tools speak — counts, bytes, journal, and baselines. Each
// test states the rejection or preview in full, because the gate is the one
// place where a silent pass costs a broken tree.

use super::support::*;
use crate::app::Exit;
use crate::core::gate::{decode_baseline, encode_baseline, Diagnostic, Level};
use crate::core::history::object;
use crate::ports::fs_fake::Memory;
use crate::ports::Fs;
use crate::adapters::store::Store;

/// One compiler-message frame, minimal but shaped like the real thing.
fn frame(code: &str, message: &str, path: &str, line: u32) -> String {
    format!(
        "{{\"reason\":\"compiler-message\",\"message\":{{\"level\":\"error\",\"message\":\"{message}\",\"code\":{{\"code\":\"{code}\"}},\"spans\":[{{\"file_name\":\"{path}\",\"line_start\":{line},\"column_start\":1,\"is_primary\":true}}]}}}}",
    )
}

fn old_error() -> Diagnostic {
    Diagnostic {
        code: "E0308".to_string(),
        level: Level::Error,
        el: None,
        owner: None,
        path: "src/other.rs".to_string(),
        line: 5,
        col: 1,
        message: "mismatched types".to_string(),
        exact: false,
    }
}

fn baseline_path(store: &Store, seq: u64) -> String {
    let entry = store.entry(seq).expect("entry").expect("present");
    format!(".el/diags/{}", object::show(&entry.to))
}

fn write_baseline(memory: &Memory, store: &Store, seq: u64, text: &str) {
    memory
        .write(&baseline_path(store, seq), text.as_bytes())
        .expect("baseline");
}

#[test]
fn a_check_rejection_reports_counts_restores_bytes_and_records_rejected() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    write_baseline(
        &memory,
        &store,
        1,
        &encode_baseline("check", &[old_error()]),
    );
    let current = format!(
        "{}\n{}",
        frame("E0308", "mismatched types", "src/other.rs", 5),
        frame("E0425", "unresolved name", "src/other.rs", 9),
    );
    let tools = Scripted::with_check(vec![current]);

    let report = run(&memory, &store, &Fixed(1), &tools, add());

    assert_eq!(report.error.as_ref().expect("refused").code, "gate.rejected");
    assert_eq!(report.exit(), Exit::Gate);
    let gate = report.gate.expect("gate");
    assert_eq!(gate.tier, "check");
    assert_eq!(gate.new, 1);
    assert_eq!(gate.existing, 1);
    assert_eq!(gate.fixed, 0);
    assert_eq!(gate.verdict, "rejected");
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    assert_eq!(store.entry(2).expect("entry").expect("present").gate, "rejected:1");
    assert_eq!(store.inflight().expect("inflight"), None);
}

#[test]
fn a_missing_baseline_is_computed_from_the_old_tree() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    let current = format!(
        "{}\n{}",
        frame("E0308", "mismatched types", "src/other.rs", 5),
        frame("E0425", "unresolved name", "src/other.rs", 9),
    );
    let old = frame("E0308", "mismatched types", "src/other.rs", 5);
    let tools = Scripted::with_check(vec![current, old]);

    let report = run(&memory, &store, &Fixed(1), &tools, add());

    assert_eq!(report.error.expect("refused").code, "gate.rejected");
    let gate = report.gate.expect("gate");
    assert_eq!(gate.new, 1);
    assert_eq!(gate.existing, 1);
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    let recorded = memory
        .read(&baseline_path(&store, 1))
        .expect("read")
        .expect("baseline recorded");
    let recorded = String::from_utf8(recorded).expect("utf8");
    assert_eq!(decode_baseline(&recorded, "check"), vec![old_error()]);
}

#[test]
fn a_dry_run_with_new_errors_exits_with_the_verdict_it_previews() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    write_baseline(
        &memory,
        &store,
        1,
        &encode_baseline("check", &[old_error()]),
    );
    let current = format!(
        "{}\n{}",
        frame("E0308", "mismatched types", "src/other.rs", 5),
        frame("E0425", "unresolved name", "src/other.rs", 9),
    );
    let tools = Scripted::with_check(vec![current]);
    let mut mutation = add();
    mutation.dry_run = true;

    let report = run(&memory, &store, &Fixed(1), &tools, mutation);

    let error = report.error.as_ref().expect("a refused preview says so");
    assert_eq!(error.code, "gate.would_reject");
    assert_eq!(report.exit(), Exit::Gate);
    let gate = report.gate.expect("gate");
    assert_eq!(gate.verdict, "preview");
    assert_eq!(gate.new, 1);
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    assert_eq!(store.entry(2).expect("entry").expect("present").gate, "preview:1");
}

#[test]
fn a_timed_out_gate_restores_and_reports_env_timeout() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());
    let before = memory.read("src/main.rs").expect("read").expect("file");
    let mut tools = Scripted::clean();
    tools.timed_out = true;

    let report = run(&memory, &store, &Fixed(1), &tools, add());

    assert_eq!(report.error.as_ref().expect("timeout").code, "env.timeout");
    assert_eq!(report.exit(), Exit::Env);
    assert_eq!(store.head().expect("head"), Some(1));
    assert_eq!(memory.read("src/main.rs").expect("read").expect("file"), before);
    assert_eq!(store.inflight().expect("inflight"), None);
}

#[test]
fn a_clean_check_commits_and_writes_the_new_baseline() {
    let memory = Memory::new();
    let store = Store::new(&memory);
    seed(&memory, &store, &catalog());

    let report = run(&memory, &store, &Fixed(1), &Scripted::clean(), add());

    assert!(report.error.is_none(), "{:?}", report.error);
    let gate = report.gate.expect("gate");
    assert_eq!(gate.tier, "check");
    assert_eq!(gate.verdict, "ok");
    assert_eq!(gate.new, 0);
    let recorded = memory
        .read(&baseline_path(&store, 2))
        .expect("read")
        .expect("baseline written");
    let recorded = String::from_utf8(recorded).expect("utf8");
    assert!(decode_baseline(&recorded, "check").is_empty());
}
