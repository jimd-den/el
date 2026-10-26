// The journal half of the store: one record per transaction, read oldest first.
//
// It lives beside `store` rather than inside it because the journal answer and the
// blob answer change for different reasons: a new record field touches this file,
// a new digest or tree rule touches `history`. The parent module owns `.el/`; this
// one owns what a record means in it.
//
// It deliberately does not verify the trees a record names. That is the
// transaction's job, and a journal that read every blob to answer `el log` would
// make the cheapest command the most expensive.

use crate::core::history::entry::Entry;
use super::Store;
use super::Error;

impl Store<'_> {
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

    /// The catalog at one sequence. New stores write it beside the tree; a store
    /// created before M8 answers `None` and the caller must rebuild or stay at head.
    pub fn put_catalog(&self, seq: u64, text: &str) -> Result<(), Error> {
        self.write_text(&Self::at(&format!("catalogs/{seq:06}")), text)
    }

    pub fn catalog_at(&self, seq: u64) -> Result<Option<String>, Error> {
        self.text(&Self::at(&format!("catalogs/{seq:06}")))
    }

    /// Entries moved aside by undo + a new mutation, newest first.
    pub fn attic_entries(&self) -> Result<Vec<Entry>, Error> {
        let mut found = Vec::new();
        for path in self.fs.list(&Self::at("attic"))? {
            let text = self.text(&path)?.ok_or_else(|| Error::Missing(path.clone()))?;
            found.push(Entry::decode(&text).map_err(Error::Malformed)?);
        }
        found.sort_by_key(|entry| entry.seq);
        Ok(found)
    }

    /// Move every journal entry after `head` to `attic/`, preserving its bytes.
    pub fn truncate_future(&self, head: u64) -> Result<(), Error> {
        for path in self.fs.list(&Self::at("journal"))? {
            let seq = path
                .rsplit('/')
                .next()
                .and_then(|name| name.parse::<u64>().ok())
                .unwrap_or(0);
            if seq <= head {
                continue;
            }
            let text = self.text(&path)?.ok_or_else(|| Error::Missing(path.clone()))?;
            self.write_text(&Self::at(&format!("attic/{seq:06}")), &text)?;
            self.fs.remove(&path)?;
        }
        Ok(())
    }
}
