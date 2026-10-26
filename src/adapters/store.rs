// The store, composed: `history`'s formats over `ports::Fs`'s bytes. This is the one
// place that knows `.el/` exists, and the one place `FR-33` lives — a blob is
// verified as it is read, every read, so no caller can forget.
//
// ADR-0018 predicted this module: verify-on-read had exactly one call site (an
// acceptance test composing `Fs::read` + `object::verify` by hand), and the signal
// that it needed a home was a second one. The transaction is that second call site.
//
// Shape: `Store::new(&dyn Fs)` where the `Fs` is rooted at the crate, so a store path
// is `.el/<rest>` and a projection path is just `<path>`. That is why there is one
// filesystem and not two: the distinction is a prefix, not a mount.
//
// It deliberately does not order anything. Which write happens before which is the
// transaction's argument (`GATE.md` §3), and a store that decided it would be a
// transaction with a filesystem attached.

use super::super::core::history::entry::{self, Entry};
use super::super::core::history::object::{self, Digest};
use super::super::core::history::tree::{self, Tree};
use super::super::ports::fs;
use std::fmt;

/// Where the store lives inside a crate (`HISTORY.md` §2).
pub const DIR: &str = ".el";

/// Anything a store operation can refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Fs(fs::Error),
    /// A blob's bytes are not the bytes its digest named (`FR-33`).
    Corrupt(object::Corrupt),
    /// A journal line that does not read back as what it says.
    Malformed(entry::Malformed),
    /// A path the store was told to read and does not have. Never a silent `None`:
    /// a missing object is a store that lost data, not a store with less content.
    Missing(String),
    /// Bytes that are not UTF-8 where the format is text.
    NotText(String),
    /// A tree with two entries for one path.
    Duplicate(tree::Duplicate),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Fs(error) => write!(f, "{error}"),
            Error::Corrupt(corrupt) => write!(f, "{corrupt}"),
            Error::Malformed(malformed) => write!(f, "{malformed}"),
            Error::Missing(path) => write!(f, "env.corrupt: {path} is missing from the store"),
            Error::NotText(path) => write!(f, "env.corrupt: {path} is not UTF-8"),
            Error::Duplicate(duplicate) => write!(f, "{duplicate}"),
        }
    }
}

impl From<fs::Error> for Error {
    fn from(error: fs::Error) -> Self {
        Error::Fs(error)
    }
}

/// A content-addressed store rooted at one crate.
pub struct Store<'a> {
    fs: &'a dyn fs::Fs,
}

impl<'a> Store<'a> {
    pub fn new(fs: &'a dyn fs::Fs) -> Self {
        Self { fs }
    }

    /// `.el/<rest>` — the only place the directory name appears.
    fn at(rest: &str) -> String {
        format!("{DIR}/{rest}")
    }

    fn text(&self, path: &str) -> Result<Option<String>, Error> {
        match self.fs.read(path)? {
            None => Ok(None),
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| Error::NotText(path.to_string())),
        }
    }

    fn write_text(&self, path: &str, text: &str) -> Result<(), Error> {
        Ok(self.fs.write(path, text.as_bytes())?)
    }

    fn required(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.fs
            .read(path)?
            .ok_or_else(|| Error::Missing(path.to_string()))
    }

    // ---- blobs ----------------------------------------------------------

    /// Store `bytes` and return the digest that names them.
    ///
    /// Idempotent: identical content is one object, so a second write is a no-op
    /// rather than a rewrite. Content is never rewritten (`HISTORY.md` §3).
    pub fn put_blob(&self, bytes: &[u8]) -> Result<Digest, Error> {
        let digest = object::of(bytes);
        let path = Self::at(&object::path(&digest));
        if !self.fs.exists(&path)? {
            self.fs.write(&path, bytes)?;
        }
        Ok(digest)
    }

    /// The bytes `digest` names, verified against it.
    ///
    /// Contract: `Err(Corrupt)` naming the path and both digests when the bytes do
    /// not match. Callers must not use the bytes on `Err`.
    pub fn get_blob(&self, digest: &Digest) -> Result<Vec<u8>, Error> {
        let path = Self::at(&object::path(digest));
        let bytes = self.required(&path)?;
        object::verify(&path, &bytes, digest).map_err(Error::Corrupt)?;
        Ok(bytes)
    }

    // ---- trees ----------------------------------------------------------

    pub fn put_tree(&self, tree: &Tree) -> Result<Digest, Error> {
        self.put_blob(tree.encode().as_bytes())
    }

    pub fn get_tree(&self, digest: &Digest) -> Result<Tree, Error> {
        let path = Self::at(&object::path(digest));
        let bytes = self.get_blob(digest)?;
        let text = String::from_utf8(bytes).map_err(|_| Error::NotText(path.clone()))?;
        Tree::decode(&text).ok_or_else(|| Error::Missing(format!("a readable tree at {path}")))
    }

    // ---- journal --------------------------------------------------------

    /// `journal/000042`, carrying `entry`, replacing whatever was there.
    ///
    /// Rewriting is how a verdict is recorded (`gate=pending` becomes `ok` or
    /// `rejected:<n>`); the sequence number is the only thing that addresses it.
    pub fn put_entry(&self, entry: &Entry) -> Result<(), Error> {
        self.write_text(&Self::at(&format!("journal/{:06}", entry.seq)), &entry.encode())
    }

    /// Every entry, oldest first.
    ///
    /// Sorted by path, which is why the sequence number is zero-padded: lexical
    /// order is numeric order, and a journal that read in `readdir` order would
    /// make `el log` depend on the filesystem.
    pub fn entries(&self) -> Result<Vec<Entry>, Error> {
        let mut found = Vec::new();
        for path in self.fs.list(&Self::at("journal"))? {
            let text = self.text(&path)?.ok_or_else(|| Error::Missing(path.clone()))?;
            found.push(Entry::decode(&text).map_err(Error::Malformed)?);
        }
        found.sort_by_key(|entry| entry.seq);
        Ok(found)
    }

    pub fn entry(&self, seq: u64) -> Result<Option<Entry>, Error> {
        let path = Self::at(&format!("journal/{seq:06}"));
        match self.text(&path)? {
            None => Ok(None),
            Some(text) => Entry::decode(&text).map(Some).map_err(Error::Malformed),
        }
    }

    // ---- head, version, inflight ---------------------------------------

    pub fn head(&self) -> Result<Option<u64>, Error> {
        let path = Self::at("head");
        match self.text(&path)? {
            None => Ok(None),
            Some(text) => text
                .trim()
                .parse()
                .map(Some)
                .map_err(|_| Error::Missing(format!("a sequence number in {path}"))),
        }
    }

    pub fn set_head(&self, seq: u64) -> Result<(), Error> {
        self.write_text(&Self::at("head"), &format!("{seq}\n"))
    }

    /// Write the format version. Present means "this is a store"; absent means the
    /// directory is not one yet, which `el new` fixes and everything else refuses.
    pub fn stamp_version(&self) -> Result<(), Error> {
        self.write_text(&Self::at("version"), "el1\n")
    }

    pub fn version(&self) -> Result<Option<String>, Error> {
        Ok(self.text(&Self::at("version"))?.map(|text| text.trim().to_string()))
    }

    /// The catalog, as the text form `ADR-0015` fixes.
    pub fn catalog(&self) -> Result<Option<String>, Error> {
        self.text(&Self::at("catalog"))
    }

    pub fn set_catalog(&self, text: &str) -> Result<(), Error> {
        self.write_text(&Self::at("catalog"), text)
    }

    /// `.el/inflight` exists only between step 6 and step 10 of a transaction.
    pub fn set_inflight(&self, marker: &str) -> Result<(), Error> {
        self.write_text(&Self::at("inflight"), marker)
    }

    pub fn inflight(&self) -> Result<Option<String>, Error> {
        self.text(&Self::at("inflight")).map(|text| text.map(|t| t.trim().to_string()))
    }

    pub fn clear_inflight(&self) -> Result<(), Error> {
        Ok(self.fs.remove(&Self::at("inflight"))?)
    }
}
