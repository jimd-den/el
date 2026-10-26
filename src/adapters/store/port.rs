// `Store` as the application's `Journal`. The inherent methods stay where they are
// (tests and the M5 boundary use them directly); this file only translates their
// error enum into the port's stable code, so `app` never sees an adapter type.
//
// It deliberately adds no behaviour: every method is one delegation, and a method
// that did more would be a second store with the first one's name.

use super::{Error, Store};
use crate::core::history::entry::Entry;
use crate::core::history::object::Digest;
use crate::core::history::tree::Tree;
use crate::ports::journal::{self, Journal};

fn port(error: Error) -> journal::Error {
    let code = if matches!(error, Error::Fs(_)) {
        "env.io"
    } else {
        "env.corrupt"
    };
    journal::Error::new(code, error.to_string())
}

impl Journal for Store<'_> {
    fn head(&self) -> Result<Option<u64>, journal::Error> {
        Store::head(self).map_err(port)
    }

    fn set_head(&self, seq: u64) -> Result<(), journal::Error> {
        Store::set_head(self, seq).map_err(port)
    }

    fn put_blob(&self, bytes: &[u8]) -> Result<Digest, journal::Error> {
        Store::put_blob(self, bytes).map_err(port)
    }

    fn get_blob(&self, digest: &Digest) -> Result<Vec<u8>, journal::Error> {
        Store::get_blob(self, digest).map_err(port)
    }

    fn put_tree(&self, tree: &Tree) -> Result<Digest, journal::Error> {
        Store::put_tree(self, tree).map_err(port)
    }

    fn get_tree(&self, digest: &Digest) -> Result<Tree, journal::Error> {
        Store::get_tree(self, digest).map_err(port)
    }

    fn put_entry(&self, entry: &Entry) -> Result<(), journal::Error> {
        Store::put_entry(self, entry).map_err(port)
    }

    fn entry(&self, seq: u64) -> Result<Option<Entry>, journal::Error> {
        Store::entry(self, seq).map_err(port)
    }

    fn entries(&self) -> Result<Vec<Entry>, journal::Error> {
        Store::entries(self).map_err(port)
    }

    fn catalog(&self) -> Result<Option<String>, journal::Error> {
        Store::catalog(self).map_err(port)
    }

    fn set_catalog(&self, text: &str) -> Result<(), journal::Error> {
        Store::set_catalog(self, text).map_err(port)
    }

    fn version(&self) -> Result<Option<String>, journal::Error> {
        Store::version(self).map_err(port)
    }

    fn stamp_version(&self) -> Result<(), journal::Error> {
        Store::stamp_version(self).map_err(port)
    }

    fn inflight(&self) -> Result<Option<String>, journal::Error> {
        Store::inflight(self).map_err(port)
    }

    fn set_inflight(&self, marker: &str) -> Result<(), journal::Error> {
        Store::set_inflight(self, marker).map_err(port)
    }

    fn clear_inflight(&self) -> Result<(), journal::Error> {
        Store::clear_inflight(self).map_err(port)
    }
}
