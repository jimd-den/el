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
    env.journal.set_catalog(&catalog.encode()).map_err(error::store)?;
    env.journal.put_entry(entry).map_err(error::store)?;
    env.journal.clear_inflight().map_err(error::store)
}
