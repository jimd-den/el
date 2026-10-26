// FR-38 / M8's repair criterion, in the form a test can make deterministic: an
// inflight marker plus a half-written file is the state a SIGKILL leaves. The
// doctor command must put head's tree back before reporting.

use crate::program::{el, ok, build, files};

#[test]
fn repair_restores_the_head_tree_after_an_interrupted_transaction() {
    let built = build("repair");
    let before = files(&built.hello);
    std::fs::write(built.hello.join(".el/inflight"), b"99 deadbeef 1\n").expect("inflight");
    std::fs::write(built.hello.join("src/main.rs"), b"half-written\n").expect("damage");

    let output = el(&built.hello, &["doctor", "--repair"]);

    ok(&output, "doctor --repair");
    assert_eq!(files(&built.hello), before);
    assert!(!built.hello.join(".el/inflight").exists());
}
