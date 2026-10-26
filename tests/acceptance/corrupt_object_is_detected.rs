// FR-33: a store is trusted only after it has been checked. This is M5's RED test,
// and it is the one that matters: a bad disk, a bad copy or a bad merge turns into a
// named `env.corrupt` that says which two digests disagree, instead of a restore that
// silently hands the compiler the wrong source.
//
// The boundary is the real adapter on a real temporary store, because "we verify on
// read" is a claim about bytes that came back off a filesystem. A fake `Fs` would
// prove that the fake was honest.

use el::adapters::fs_local::Local;
use el::core::history::object::{self, Digest};
use el::ports::Fs;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = "target/acceptance/corrupt-store";

/// A clean store root for this test, with its top-level directory made.
fn root() -> PathBuf {
    let root = Path::new(ROOT).to_path_buf();
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("create store root");
    root
}

/// Store `bytes` the way a transaction would: path is the digest.
fn store(fs: &Local, bytes: &[u8]) -> Digest {
    let digest = object::of(bytes);
    fs.write(&object::path(&digest), bytes).expect("write blob");
    digest
}

/// Flip the lowest bit of one byte in the file on disk, as rot would.
fn corrupt(root: &Path, digest: &Digest) {
    let path = root.join(object::path(digest));
    let mut bytes = fs::read(&path).expect("read on disk");
    bytes[2] ^= 0b0000_0001;
    fs::write(&path, bytes).expect("write on disk");
}

#[test]
fn a_flipped_byte_in_a_stored_blob_is_a_named_error_and_not_a_restore() {
    let root = root();
    let fs = Local::new(&root);
    let bytes = b"pub fn clamp(self, max: usize) -> Self {\n    Self(self.0.min(max))\n}\n";
    let digest = store(&fs, bytes);
    corrupt(&root, &digest);

    let read_back = fs
        .read(&object::path(&digest))
        .expect("the file is there")
        .expect("the file has bytes");
    let error = object::verify(&object::path(&digest), &read_back, &digest)
        .expect_err("one flipped byte is not the blob the digest names");

    assert!(
        error.to_string().starts_with("env.corrupt: objects/"),
        "{error}"
    );
    assert!(
        error.to_string().contains(&object::show(&digest)),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .contains(&object::show(&object::of(&read_back))),
        "{error}"
    );
    assert_ne!(object::show(&error.expected), object::show(&error.actual));
}

#[test]
fn an_intact_blob_reads_back_byte_identically_so_the_check_above_can_fail() {
    let root = root();
    let fs = Local::new(&root);
    let bytes = b"mod clamp;\n";
    let digest = store(&fs, bytes);

    let read_back = fs
        .read(&object::path(&digest))
        .expect("read")
        .expect("bytes");

    assert_eq!(read_back, bytes);
    assert_eq!(
        object::verify(&object::path(&digest), &read_back, &digest),
        Ok(())
    );
}

#[test]
fn an_empty_blob_is_stored_and_read_back_without_a_special_case() {
    let root = root();
    let fs = Local::new(&root);
    let digest = store(&fs, b"");

    let read_back = fs.read(&object::path(&digest)).expect("read");

    assert_eq!(read_back.as_deref(), Some(&b""[..]));
    assert_eq!(
        object::verify(&object::path(&digest), &read_back.expect("bytes"), &digest),
        Ok(())
    );
}
