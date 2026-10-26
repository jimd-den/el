// The crash contract of `ports::Fs::write`, proved where it lives.
//
// `HISTORY.md` §7 orders every transaction so that a crash can only ever leave the
// old tree or the new one. The mechanism is tmp → fsync → rename, and the property
// that matters is negative: after a write that does not reach the rename, the old
// bytes are still exactly what a reader sees. A unit test cannot say this (it would
// need a disk), and a test that killed a process would be flaky, so the interruption
// is simulated at the one place a real crash could stop it — the staging directory.
//
// The second half is path safety: store paths come off disk, so `..` and absolute
// paths are untrusted input, not programmer error.

use crate::helpers::fresh_dir;
use el::adapters::fs_local::Local;
use el::ports::{Fs, Kind};
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn a_write_that_cannot_stage_leaves_the_previous_bytes_intact() {
    let root = fresh_dir("fs-atomic");
    let store = Local::new(&root);
    store.write("head", b"41\n").expect("first write");
    let staging = root.join("tmp");
    fs::create_dir_all(&staging).expect("mkdir tmp");
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o500)).expect("read-only tmp");

    let refused = store.write("head", b"42\n");

    fs::set_permissions(&staging, fs::Permissions::from_mode(0o700)).expect("restore");
    assert!(
        refused.is_err(),
        "an unwritable staging directory must refuse"
    );
    assert_eq!(
        store.read("head").expect("read").as_deref(),
        Some(&b"41\n"[..])
    );
}

#[test]
fn a_successful_write_replaces_the_whole_file_rather_than_appending() {
    let store = Local::new(fresh_dir("fs-replace"));
    store.write("head", b"41\n").expect("first");

    store.write("head", b"7\n").expect("second");

    assert_eq!(
        store.read("head").expect("read").as_deref(),
        Some(&b"7\n"[..])
    );
}

#[test]
fn a_path_that_would_leave_the_store_is_refused() {
    let root = fresh_dir("fs-escape").join("store");
    let store = Local::new(&root);

    for path in ["../escape", "/etc/passwd", "objects/../../escape", ""] {
        assert_eq!(
            store.write(path, b"x").map_err(|error| error.kind),
            Err(Kind::NotAFile),
            "{path:?} was not refused"
        );
    }
    assert!(!root.parent().expect("parent").join("escape").exists());
}

#[test]
fn a_write_creates_the_directories_a_digest_path_needs() {
    let store = Local::new(fresh_dir("fs-parents"));

    store.write("objects/ab/cdef", b"blob").expect("write");

    assert!(store.root().join("objects/ab/cdef").is_file());
    assert_eq!(
        store.list("objects").expect("list"),
        vec!["objects/ab/cdef".to_string()]
    );
}

#[test]
fn reading_a_directory_is_an_error_and_not_bytes() {
    let root = fresh_dir("fs-not-a-file");
    let store = Local::new(&root);
    store.write("objects/ab/cdef", b"blob").expect("write");

    let error = store
        .read("objects/ab")
        .expect_err("a directory has no bytes");

    assert_eq!(error.path, "objects/ab");
    assert_ne!(error.kind, Kind::Missing);
}
