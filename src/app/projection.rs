// Getting the projection onto disk, and back off it. A tree is content-addressed;
// a projection is the bytes a compiler reads. This module is the only place the
// two are compared, which is why a restore and a materialize cannot disagree
// about which paths `el` owns.
//
// It deliberately does not decide *whether* to write: the transaction orders the
// calls, because the order is the crash contract (`GATE.md` §3).

use super::error;
use super::report::Error;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::render::{Projection, render};
use std::collections::BTreeSet;

/// Store every file's bytes and the tree that names them.
pub fn tree_of(env: &super::Env<'_>, projection: &Projection) -> Result<Tree, Error> {
    let mut entries = Vec::with_capacity(projection.files.len());
    for file in &projection.files {
        let digest = env
            .journal
            .put_blob(file.text.as_bytes())
            .map_err(error::store)?;
        entries.push((file.path.clone(), digest));
    }
    let tree = Tree::new(entries)
        .map_err(|duplicate| error::corrupt(duplicate.to_string()))?;
    env.journal.put_tree(&tree).map_err(error::store)?;
    Ok(tree)
}

/// Write the paths that changed, and remove the paths that disappeared.
///
/// Contract: a path present in both trees with the same digest is not touched,
/// so an edit does not churn every file's mtime.
/// How many files one materialize wrote, split by new and rewritten, so a
/// committed change can message its own footprint.
pub struct Written {
    pub added: usize,
    pub changed: usize,
}

impl Written {
    pub fn message(&self) -> String {
        let total = self.added + self.changed;
        let files = if total == 1 { "file" } else { "files" };
        if self.added == 0 {
            format!("el: rewrote {total} {files}")
        } else if self.changed == 0 {
            format!("el: wrote {total} {files}")
        } else {
            format!("el: wrote {total} {files} ({} new, {} rewritten)", self.added, self.changed)
        }
    }
}

pub fn materialize(
    env: &super::Env<'_>,
    projection: &Projection,
    old: &Tree,
    new: &Tree,
) -> Result<Written, Error> {
    let diff = old.diff(new);
    let written = Written { added: diff.added.len(), changed: diff.changed.len() };

    for path in diff.added.iter().chain(diff.changed.iter()) {
        let file = projection
            .file(path)
            .ok_or_else(|| error::corrupt(format!("the tree names {path}, but it was not rendered")))?;
        env.fs
            .write(path, file.text.as_bytes())
            .map_err(error::filesystem)?;
    }
    for path in &diff.removed {
        env.fs.remove(path).map_err(error::filesystem)?;
    }
    Ok(written)
}

/// Put `tree` back on disk exactly: write every blob, then remove every owned
/// path the tree does not name. Only `src/**` and `Cargo.toml` are owned.
pub fn restore(env: &super::Env<'_>, tree: &Tree) -> Result<(), Error> {
    for (path, digest) in tree.entries() {
        let bytes = env.journal.get_blob(digest).map_err(error::store)?;
        env.fs.write(path, &bytes).map_err(error::filesystem)?;
    }

    let known: BTreeSet<&str> = tree.paths().collect();
    for path in env.fs.list("src").map_err(error::filesystem)? {
        if !known.contains(path.as_str()) {
            env.fs.remove(&path).map_err(error::filesystem)?;
        }
    }
    if !known.contains("Cargo.toml") {
        env.fs.remove("Cargo.toml").map_err(error::filesystem)?;
    }
    Ok(())
}

/// Compare the owned files on disk with what the catalog says they should be.
///
/// A whitespace-only difference is what `cargo fmt` produces, so it is rewritten
/// with one note (ADR-0009). Anything else is drift and exit 4: the catalog would
/// otherwise silently overwrite a human's edit.
pub fn check_drift(
    env: &super::Env<'_>,
    catalog: &Catalog,
    crate_name: &str,
) -> Result<Vec<String>, Error> {
    let projection = render(catalog, crate_name);
    let mut notes = Vec::new();

    for file in &projection.files {
        let Some(bytes) = env.fs.read(&file.path).map_err(error::filesystem)? else {
            return Err(drift(&file.path, "is missing"));
        };
        let Ok(text) = String::from_utf8(bytes) else {
            return Err(drift(&file.path, "is not UTF-8"));
        };
        if text == file.text {
            continue;
        }
        if only_whitespace(&text, &file.text) {
            env.fs
                .write(&file.path, file.text.as_bytes())
                .map_err(error::filesystem)?;
            notes.push(format!("el: {} differed by whitespace; re-rendered it", file.path));
        } else {
            return Err(drift(&file.path, "differs from the catalog; restore it before changing it"));
        }
    }
    Ok(notes)
}

fn drift(path: &str, why: &str) -> Error {
    Error {
        code: "env.drift",
        exit: super::Exit::Env,
        message: format!("{path} {why} (`el undo` restores the last committed tree)"),
    }
}

fn only_whitespace(left: &str, right: &str) -> bool {
    left.split_whitespace().eq(right.split_whitespace())
}
