// Preparing a mutation: plan before any write. This is the read-only half
// of the transaction — load the catalog, choose the tier, check for drift,
// plan every change against the policy — so `apply` never plans after it
// has already written. A refusal here is a value with nothing to undo.

use super::gate;
use super::load;
use super::outcome;
use super::projection;
use super::stage;
use super::super::{change, Env};
use super::{error, Mutation};
use crate::core::history::object::Digest;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::render::{render, Projection};

pub(super) struct Prepared {
    pub(super) seq: u64,
    pub(super) old_tree: Tree,
    pub(super) old_digest: Digest,
    pub(super) new_tree: Tree,
    pub(super) new_digest: Digest,
    pub(super) projection: Projection,
    pub(super) catalog: Catalog,
    pub(super) old_catalog: Catalog,
    pub(super) tier: crate::core::gate::Tier,
    pub(super) changed: Vec<crate::app::report::Changed>,
}

pub(super) fn prepare(env: &Env<'_>, mutation: &Mutation, notes: &mut Vec<String>) -> Result<Prepared, error::Error> {
    load::ensure_crate(env)?;
    let catalog = load::catalog(env)?;
    let policy = load::policy(env)?;
    let tier = gate::select(&policy.gate, mutation.fast, mutation.strict)?;
    let (head, old_tree) = load::head_tree(env)?;
    // `init` commits an empty tree; the first `new` transaction is what writes the
    // projection, so there is nothing on disk to compare yet.
    if !old_tree.is_empty() {
        notes.extend(projection::check_drift(env, &catalog, &env.crate_name)?);
    }

    let mut working = catalog.clone();
    let mut projection = render(&working, &env.crate_name);
    let mut changed = Vec::new();
    for change in &mutation.changes {
        let before = working.clone();
        let outcome = crate::core::plan::plan(
            &working,
            &crate::core::plan::Request {
                change,
                policy: &policy,
                crate_name: &env.crate_name,
            },
        )
        .map_err(outcome::violation_error)?;
        changed.extend(change::describe(change, &before, &outcome.catalog));
        working = outcome.catalog;
        projection = outcome.projection;
    }

    let seq = if mutation.dry_run {
        stage::next_free(env, head)?
    } else {
        env.journal.truncate_future(head).map_err(error::store)?;
        head + 1
    };
    let old_digest = env.journal.put_tree(&old_tree).map_err(error::store)?;
    let new_tree = projection::tree_of(env, &projection)?;
    let new_digest = env.journal.put_tree(&new_tree).map_err(error::store)?;
    Ok(Prepared {
        seq,
        old_tree,
        old_digest,
        new_tree,
        new_digest,
        projection,
        catalog: working,
        old_catalog: catalog,
        tier,
        changed,
    })
}
