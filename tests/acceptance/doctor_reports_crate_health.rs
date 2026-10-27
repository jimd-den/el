// COMMANDS.md §4.7 promises `doctor` reports the crate's health: drift, an
// interrupted transaction, and the store. It printed root, version, head, journal
// and attic — so a user whose projection no longer matched the catalog was told
// nothing, and the command that exists to answer "is this crate well" answered a
// smaller question than the one asked.

use crate::program::{build, el, ok};
use std::fs;

#[test]
fn a_healthy_crate_reports_no_drift_and_no_interrupted_transaction() {
    let built = build("doctor-healthy");

    let output = el(&built.hello, &["doctor"]);

    ok(&output, "doctor");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("drift"), "{text}");
    assert!(text.contains("inflight"), "{text}");
    assert!(text.contains("objects"), "{text}");
    assert!(text.contains("none"), "{text}");
}

#[test]
fn a_hand_edited_file_is_named_as_drift() {
    let built = build("doctor-drift");
    fs::write(built.hello.join("src/greet.rs"), "// mine now\n").expect("edit");

    let output = el(&built.hello, &["doctor"]);

    ok(&output, "doctor still reports rather than refuses");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("src/greet.rs"), "{text}");
    assert!(text.contains("differs"), "{text}");
}

#[test]
fn an_interrupted_transaction_is_named_with_the_command_that_clears_it() {
    let built = build("doctor-inflight");
    fs::write(built.hello.join(".el/inflight"), b"99 deadbeef 1\n").expect("inflight");

    let output = el(&built.hello, &["doctor"]);

    ok(&output, "doctor");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("inflight"), "{text}");
    assert!(text.contains("--repair"), "{text}");
}
