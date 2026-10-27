// A tree recorded before the manifest became unowned can still name Cargo.toml.
// Undoing to it must not write that historical blob over the user's current
// manifest; every other path in the tree is still restored.

use crate::helpers::fresh_dir;
use el::adapters::{Local, ProcTools, Store, SystemClock};
use el::app::{Env, projection};
use el::core::history::tree::Tree;
use el::ports::Fs;

#[test]
fn restore_leaves_the_manifest_alone() {
    let root = fresh_dir("restore-manifest");
    let local = Local::new(&root);
    let store = Store::new(&local);
    local.write("Cargo.toml", b"user manifest\n").expect("manifest");
    local.write("src/main.rs", b"old source\n").expect("source");
    let manifest = store.put_blob(b"historical manifest\n").expect("blob");
    let source = store.put_blob(b"old source\n").expect("blob");
    let tree = Tree::new(vec![
        ("Cargo.toml".to_string(), manifest),
        ("src/main.rs".to_string(), source),
    ])
    .expect("tree");
    let clock = SystemClock;
    let tools = ProcTools;
    let env = Env {
        fs: &local,
        journal: &store,
        clock: &clock,
        tools: &tools,
        actor: "test".to_string(),
        crate_name: "demo".to_string(),
        root: root.display().to_string(),
        cargo_timeout: 1,
    };

    projection::restore(&env, &tree).expect("restore");

    assert_eq!(
        local.read("Cargo.toml").expect("read").unwrap(),
        b"user manifest\n"
    );
    assert_eq!(
        local.read("src/main.rs").expect("read").unwrap(),
        b"old source\n"
    );
}
