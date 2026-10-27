// Preparing a mutation: plan before any write. This is the read-only half
// of the transaction — load the catalog, choose the tier, check for drift,
// plan every change against the policy — so `apply` never plans after it
// has already written. A refusal here is a value with nothing to undo.

use super::gate;
use super::load;
use super::projection;
use super::stage;
use super::super::{change, Env, Exit};
use super::{error, Mutation};
use crate::core::history::object::Digest;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::signature;
use crate::core::plan::Violations;
use crate::core::render::{render, Projection};

pub fn violation_error(violations: Violations, context: Option<String>) -> error::Error {
    let first = violations.first();
    let message = match context {
        Some(context) => format!("{context}: {violations}"),
        None => violations.to_string(),
    };
    error::Error {
        code: first.code.as_str(),
        exit: if first.code == crate::core::plan::Code::UnknownRef {
            Exit::NotFound
        } else {
            Exit::Violation
        },
        message,
    }
}

/// One line naming a change in a multi-change refusal, so the caller can
/// tell which item failed without an id that was never issued.
fn describe_change(index: usize, change: &Change) -> String {
    let desc = match change {
        Change::AddElement {
            kind,
            module,
            signature,
            ..
        } => {
            let name = signature::split(*kind, signature)
                .and_then(|(name, _)| name)
                .unwrap_or(signature.as_str());
            format!("{} {name} in {module}", kind.as_str())
        }
        Change::EditElement { id, .. } => format!("edit {id}"),
        Change::MoveElement { id, .. } => format!("mv {id}"),
        Change::Rename { id, .. } => format!("rename {id}"),
        Change::AddStmt { owner, .. } => format!("add stmt to {owner}"),
        Change::EditStmt { id, .. } => format!("edit stmt {id}"),
        Change::EditBody { id, .. } => format!("edit body {id}"),
        Change::ReorderStmt { id, .. } => format!("reorder stmt {id}"),
        Change::MoveStmt { id, .. } => format!("mv stmt {id}"),
        Change::Remove { id } => format!("rm {id}"),
        Change::AddDep { name } => format!("add dep {name}"),
    };
    format!("change {index} ({desc})")
}

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
    for (index, change) in mutation.changes.iter().enumerate() {
        let before = working.clone();
        let outcome = crate::core::plan::plan(
            &working,
            &crate::core::plan::Request {
                change,
                policy: &policy,
                crate_name: &env.crate_name,
            },
        )
        .map_err(|violations| {
            let context = (mutation.changes.len() > 1 || !mutation.origins.is_empty())
                .then(|| {
                let described = describe_change(index, change);
                    match mutation.origins.get(index) {
                        Some(line) if *line > 0 => format!("apply line {line}: {described}"),
                        _ => described,
                    }
                });
            violation_error(violations, context)
        })?;
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
