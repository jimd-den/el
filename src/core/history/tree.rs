// A tree is the definition of the projection: which paths `el` owns and which blob
// each one holds (`HISTORY.md` §4.2). Undo materializes a tree, so anything not
// listed here is a file `el` never touches — the user's `README.md`, `target/`,
// `.git/`.
//
// Shape: a sorted list of `(path, digest)`, encoded one `path\tdigest` line at a
// time. Sorted because two trees with the same content must encode to the same bytes,
// or content addressing would give one tree many names (the same reason `render`
// sorts).
//
// It deliberately does not store file modes, symlinks or directories: a projection is
// generated source plus a manifest, and a mode bit cannot be represented in the
// catalog that produced it. If `el` ever owns an executable, that decision needs a new
// field, not a guess here.

use super::object::{self, Digest};
use std::fmt;

/// The projection, as the store sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tree {
    /// Sorted by path, unique by path. The order is part of the value, not of the
    /// caller's convenience.
    entries: Vec<(String, Digest)>,
}

/// What changed between two trees, from the caller's point of view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diff {
    /// Paths in `other` that `self` did not have.
    pub added: Vec<String>,
    /// Paths in `self` that `other` did not have.
    pub removed: Vec<String>,
    /// Paths in both, with different blobs.
    pub changed: Vec<String>,
}

impl Diff {
    /// Whether the two trees describe the same projection.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

impl Tree {
    /// A tree from `(path, digest)` pairs, sorted and refused when a path repeats.
    ///
    /// A repeated path is refused rather than resolved: two blobs for one path is a
    /// store that cannot say what the projection is, and picking the last one would
    /// make the answer depend on iteration order.
    pub fn new(entries: Vec<(String, Digest)>) -> Result<Self, Duplicate> {
        let mut sorted = entries;
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some(pair) = sorted.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(Duplicate {
                path: pair[0].0.clone(),
            });
        }
        Ok(Self { entries: sorted })
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(path, _)| path.as_str())
    }

    /// The entries in sorted order. A materializer needs the digest too; exposing
    /// only paths would make it look every blob up by path, which is not the key.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Digest)> {
        self.entries.iter().map(|(path, digest)| (path.as_str(), digest))
    }

    pub fn digest_of(&self, path: &str) -> Option<&Digest> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == path)
            .map(|(_, digest)| digest)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The record form: one `path\tdigest` line per entry, sorted, newline-terminated.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        for (path, digest) in &self.entries {
            out.push_str(path);
            out.push('\t');
            out.push_str(&object::show(digest));
            out.push('\n');
        }
        out
    }

    /// The inverse of `encode`; `None` for a line that is not `path\tdigest`, a
    /// digest that is not a digest, or a path that repeats.
    pub fn decode(text: &str) -> Option<Self> {
        let mut entries = Vec::new();
        for line in text.lines() {
            let (path, digest) = line.split_once('\t')?;
            if path.is_empty() {
                return None;
            }
            entries.push((path.to_string(), object::parse(digest)?));
        }
        Self::new(entries).ok()
    }

    /// What changed between `self` and `other`, by path.
    pub fn diff(&self, other: &Tree) -> Diff {
        let mut diff = Diff::default();

        for (path, digest) in &self.entries {
            match other.digest_of(path) {
                None => diff.removed.push(path.clone()),
                Some(theirs) if theirs != digest => diff.changed.push(path.clone()),
                Some(_) => {}
            }
        }
        for (path, _) in &other.entries {
            if self.digest_of(path).is_none() {
                diff.added.push(path.clone());
            }
        }
        diff
    }
}

/// Two entries for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate {
    pub path: String,
}

impl fmt::Display for Duplicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "env.corrupt: tree has two entries for {}", self.path)
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;
