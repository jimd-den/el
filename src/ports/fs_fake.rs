// An in-memory `Fs`, for the two questions a real disk cannot answer: *in what
// order* did the store touch things, and what did it refuse to touch at all.
//
// Test-only (`#[cfg(test)]` in `ports/mod.rs`), and deliberately dumb: it holds
// bytes and records calls. It does not model fsync, permissions or atomicity — those
// are the adapter's promises, proved in `tests/integration` against a real
// filesystem. A fake that claimed to be atomic would be a fake proving its own claim.

use super::fs::{Error, Fs, Kind};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// Files in memory, plus a log of every call, in order.
#[derive(Debug, Default)]
pub struct Memory {
    files: RefCell<BTreeMap<String, Vec<u8>>>,
    log: RefCell<Vec<String>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that already holds `files`.
    pub fn with(files: &[(&str, &[u8])]) -> Self {
        let memory = Self::new();
        for (path, bytes) in files {
            memory
                .files
                .borrow_mut()
                .insert((*path).to_string(), bytes.to_vec());
        }
        memory
    }

    /// Every call made against this fake, in order: `write <path>`, `read <path>`, …
    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    /// The paths written, in order. The transaction's step order is a claim about
    /// exactly this list.
    pub fn writes(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .filter_map(|line| line.strip_prefix("write ").map(str::to_string))
            .collect()
    }

    pub fn paths(&self) -> Vec<String> {
        self.files.borrow().keys().cloned().collect()
    }

    pub fn note(&self, what: &str) {
        self.log.borrow_mut().push(what.to_string());
    }

    fn record(&self, verb: &str, path: &str) {
        self.note(&format!("{verb} {path}"));
    }
}

impl Fs for Memory {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.record("read", path);
        Ok(self.files.borrow().get(path).cloned())
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Error> {
        self.record("write", path);
        self.files.borrow_mut().insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), Error> {
        self.record("remove", path);
        self.files.borrow_mut().remove(path);
        Ok(())
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, Error> {
        self.record("list", prefix);
        let mut found: Vec<String> = self
            .files
            .borrow()
            .keys()
            .filter(|path| path.starts_with(prefix))
            .cloned()
            .collect();
        found.sort();
        Ok(found)
    }

    fn exists(&self, path: &str) -> Result<bool, Error> {
        self.record("exists", path);
        Ok(self.files.borrow().contains_key(path))
    }
}

/// Refuse any write: the store that a read-only crate is, and the shortest way to
/// prove a violation wrote nothing.
#[derive(Debug, Default)]
pub struct ReadOnly {
    inner: Memory,
}

impl ReadOnly {
    pub fn of(memory: Memory) -> Self {
        Self { inner: memory }
    }

    pub fn log(&self) -> Vec<String> {
        self.inner.log()
    }
}

impl Fs for ReadOnly {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        self.inner.read(path)
    }

    fn write(&self, path: &str, _bytes: &[u8]) -> Result<(), Error> {
        self.inner.record("write-denied", path);
        Err(Error::new(path, Kind::Denied))
    }

    fn remove(&self, path: &str) -> Result<(), Error> {
        self.inner.record("remove-denied", path);
        Err(Error::new(path, Kind::Denied))
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, Error> {
        self.inner.list(prefix)
    }

    fn exists(&self, path: &str) -> Result<bool, Error> {
        self.inner.exists(path)
    }
}
