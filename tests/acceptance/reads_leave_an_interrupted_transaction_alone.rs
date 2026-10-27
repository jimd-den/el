// FR-38 says an interrupted transaction is rolled back "at the next invocation".
// Read literally, that made every read verb a writer: `el ls` running beside a
// live `el add` saw the writer's inflight marker and restored head's tree over the
// half-written projection, then reported a catalog that no file matched. Recovery
// needs the writer lock, and a read must not take it — so a read reports the
// committed state and leaves the marker for `doctor --repair` or the next writer.

use crate::program::{build, el, ok};
use std::fs;

fn interrupt(built: &crate::program::Built) -> Vec<u8> {
    fs::write(built.hello.join(".el/inflight"), b"99 deadbeef 1\n").expect("inflight");
    fs::write(built.hello.join("src/main.rs"), b"half-written\n").expect("damage");
    b"half-written\n".to_vec()
}

#[test]
fn a_listing_leaves_an_interrupted_projection_alone() {
    let built = build("read-inflight-ls");
    let damaged = interrupt(&built);

    let output = el(&built.hello, &["ls"]);

    ok(&output, "ls");
    assert_eq!(fs::read(built.hello.join("src/main.rs")).expect("main"), damaged);
    assert!(built.hello.join(".el/inflight").exists(), "the read cleared the marker");
}

#[test]
fn showing_source_leaves_an_interrupted_projection_alone() {
    let built = build("read-inflight-show");
    let damaged = interrupt(&built);

    let output = el(&built.hello, &["show", &built.main, "--source"]);

    ok(&output, "show --source");
    assert_eq!(fs::read(built.hello.join("src/main.rs")).expect("main"), damaged);
    assert!(built.hello.join(".el/inflight").exists());
}

#[test]
fn log_and_diff_leave_an_interrupted_projection_alone() {
    let built = build("read-inflight-log");
    let damaged = interrupt(&built);

    ok(&el(&built.hello, &["log"]), "log");
    ok(&el(&built.hello, &["diff"]), "diff");

    assert_eq!(fs::read(built.hello.join("src/main.rs")).expect("main"), damaged);
    assert!(built.hello.join(".el/inflight").exists());
}
