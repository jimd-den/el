// FR-34: the head's catalog snapshot is the journal's catalog. Rebuild writes it
// back byte for byte, including after the cache file is corrupted.

use crate::program::{build, el, ok};

#[test]
fn rebuild_restores_the_catalog_from_the_head_snapshot() {
    let built = build("rebuild");
    let path = built.hello.join(".el/catalog");
    let original = std::fs::read(&path).expect("catalog");
    std::fs::write(&path, b"not a catalog\n").expect("corrupt");

    let output = el(&built.hello, &["doctor", "--rebuild"]);
    ok(&output, "doctor --rebuild");

    assert_eq!(std::fs::read(&path).expect("catalog"), original);
}
