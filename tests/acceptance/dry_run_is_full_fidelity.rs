// FR-23: `--dry-run` performs the whole transaction — render and gate — and
// always restores, and its exit code is the verdict, not an optimistic 0. A script
// that previews a change and is told "ok" will apply it, so the preview must be
// exactly as wrong as the real thing.

use crate::program::{build, el_stdin, files, ok};

#[test]
fn a_dry_run_that_would_compile_changes_nothing_and_exits_zero() {
    let built = build("dry-run-ok");
    let before = files(&built.hello);

    let output = el_stdin(
        &built.hello,
        &["add", "fn", "extra", "--sig", "fn extra()", "--body", "-", "--dry-run"],
        "let x = 1;",
    );

    ok(&output, "dry run");
    assert_eq!(files(&built.hello), before);
}

#[test]
fn a_dry_run_that_would_be_rejected_exits_with_the_gate_verdict() {
    let built = build("dry-run-rejected");
    let before = files(&built.hello);

    let output = el_stdin(
        &built.hello,
        &["add", "fn", "broken", "--sig", "fn broken()", "--body", "-", "--dry-run"],
        "undefined_name();",
    );

    assert_eq!(output.status.code(), Some(3), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(files(&built.hello), before);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("E0425"), "{text}");
    assert!(text.contains("would be rejected"), "{text}");
}

#[test]
fn the_same_change_without_dry_run_is_also_rejected() {
    let built = build("dry-run-real");
    let body = "undefined_name();";

    let real = el_stdin(
        &built.hello,
        &["add", "fn", "broken", "--sig", "fn broken()", "--body", "-"],
        body,
    );

    assert_eq!(real.status.code(), Some(3), "{}", String::from_utf8_lossy(&real.stderr));
}
