// The journal port: the store operations a use case needs, stated as an interface
// the application layer owns. `app` must not name `adapters::Store`; it names this,
// and the composition root hands it the concrete store.
//
// Errors are plain data with a stable code, because the application maps them to an
// exit code and an envelope field. A port that leaked an adapter's error enum would
// put the store's shape in the decision layer.
//
// It deliberately does not expose blob paths. A caller that knew where a blob lived
// could bypass verification; every read here verifies, every time (FR-33).

use crate::core::history::entry::Entry;
use crate::core::history::object::Digest;
use crate::core::history::tree::Tree;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// `env.corrupt`, `env.io`, `env.missing` — the envelope's vocabulary.
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub trait Journal {
    fn head(&self) -> Result<Option<u64>, Error>;
    fn set_head(&self, seq: u64) -> Result<(), Error>;

    fn put_blob(&self, bytes: &[u8]) -> Result<Digest, Error>;
    fn get_blob(&self, digest: &Digest) -> Result<Vec<u8>, Error>;
    fn put_tree(&self, tree: &Tree) -> Result<Digest, Error>;
    fn get_tree(&self, digest: &Digest) -> Result<Tree, Error>;

    fn put_entry(&self, entry: &Entry) -> Result<(), Error>;
    fn entry(&self, seq: u64) -> Result<Option<Entry>, Error>;
    fn entries(&self) -> Result<Vec<Entry>, Error>;

    fn catalog(&self) -> Result<Option<String>, Error>;
    fn set_catalog(&self, text: &str) -> Result<(), Error>;
    fn version(&self) -> Result<Option<String>, Error>;
    fn stamp_version(&self) -> Result<(), Error>;

    fn inflight(&self) -> Result<Option<String>, Error>;
    fn set_inflight(&self, marker: &str) -> Result<(), Error>;
    fn clear_inflight(&self) -> Result<(), Error>;
}
