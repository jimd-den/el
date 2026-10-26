// A public repository with no LICENSE is "all rights reserved" by default: nobody
// may legally copy, fork, or depend on it, which is the opposite of what an open
// project intends. `LICENSE` alone is not enough either — the manifest is what
// every registry and dependency scanner actually reads, so the two must agree.
//
// This is a test rather than a promise because the failure mode is silent: the
// file is committed once and then nobody ever looks at it again.

use std::path::Path;

const DECLARED: &str = "MIT";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> String {
    std::fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml")
}

#[test]
fn the_repository_has_a_licence_file() {
    let path = root().join("LICENSE");

    assert!(path.exists(), "no LICENSE: the project is all rights reserved");
    let text = std::fs::read_to_string(&path).expect("read LICENSE");
    assert!(
        text.contains("MIT License"),
        "LICENSE is not the MIT text this project declares"
    );
}

#[test]
fn the_manifest_declares_the_same_licence() {
    assert!(
        manifest().contains(&format!("license = \"{DECLARED}\"")),
        "Cargo.toml does not declare `license = \"{DECLARED}\"`"
    );
}
