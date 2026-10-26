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
}
