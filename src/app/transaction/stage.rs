// The two writes that bracket a gate: stage the pending entry before the
// projection changes, commit head and catalog after it passes. They are separate
// functions so the order cannot be rearranged by editing one call site.

use super::super::error;
use super::super::Env;
use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;

pub fn stage(env: &Env<'_>, entry: &Entry) -> Result<(), error::Error> {
    env.journal.put_entry(entry).map_err(error::store)
}

pub fn commit(env: &Env<'_>, seq: u64, catalog: &Catalog, entry: &Entry) -> Result<(), error::Error> {
    env.journal.set_head(seq).map_err(error::store)?;
    let text = catalog.encode();
    env.journal.set_catalog(&text).map_err(error::store)?;
    env.journal.put_catalog(seq, &text).map_err(error::store)?;
    env.journal.put_entry(entry).map_err(error::store)?;
    env.journal.clear_inflight().map_err(error::store)
}

/// The next sequence for a preview: past both the journal and the attic, so a
/// dry-run never overwrites a future that a real mutation would have moved aside.
pub(super) fn next_free(env: &Env<'_>, head: u64) -> Result<u64, error::Error> {
    let journal = env.journal.entries().map_err(error::store)?;
    let attic = env.journal.attic_entries().map_err(error::store)?;
    let max = journal
        .iter()
        .chain(attic.iter())
        .map(|entry| entry.seq)
        .max()
        .unwrap_or(head);
    Ok(max.max(head) + 1)
}
