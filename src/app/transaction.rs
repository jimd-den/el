// One mutating command, in the order `GATE.md` §3 fixes. The order is the crash
// contract: plan before any write, inflight before the first projection write, and
// head only after the gate speaks. A refusal is therefore a value with nothing to
// undo, and a crash leaves a marker the next invocation can act on.
//
// It deliberately does not parse a command line or render output. `verbs` builds
// the changes; `out_*` reads the report; this module decides only what is written
// and in what order.

use super::change;
use super::error::{self, Error};
use super::load;
use super::projection;
use super::report::Report;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::object::{self, Digest};
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::render::{render, Projection};
use std::time::Instant;

mod gate;
mod outcome;
mod stage;

#[cfg(test)]
#[path = "transaction/support.rs"]
mod support;
#[cfg(test)]
#[path = "transaction_tests.rs"]
mod transaction_tests;
#[cfg(test)]
#[path = "transaction_recovery_tests.rs"]
mod transaction_recovery_tests;

pub struct Mutation {
    pub cmd: &'static str,
    pub op: Op,
    pub target: String,
    pub message: String,
    pub changes: Vec<crate::core::model::change::Change>,
    pub dry_run: bool,
    pub strict: bool,
}

/// Roll back an interrupted transaction before the command reads anything.
pub fn recover(env: &super::Env<'_>) -> Result<Vec<String>, Error> {
    if env.journal.inflight().map_err(error::store)?.is_none() {
        return Ok(Vec::new());
    }
    let (_, tree) = load::head_tree(env)?;
    projection::restore(env, &tree)?;
    env.journal.clear_inflight().map_err(error::store)?;
    Ok(vec!["el: restored head's tree after an interrupted transaction".to_string()])
}

struct Prepared {
    seq: u64,
    old_tree: Tree,
    old_digest: Digest,
    new_tree: Tree,
    new_digest: Digest,
    projection: Projection,
    catalog: Catalog,
    changed: Vec<super::report::Changed>,
}

pub fn apply(env: &super::Env<'_>, mutation: Mutation) -> Report {
    let mut notes = Vec::new();
    let prepared = match prepare(env, &mutation, &mut notes) {
        Ok(prepared) => prepared,
        Err(error) => return Report::fail(mutation.cmd, error),
    };
    let started = Instant::now();
    let marker = format!(
        "{} {} {}",
        prepared.seq,
        object::show(&prepared.old_digest),
        env.clock.now_unix()
    );
    if let Err(error) = env.journal.set_inflight(&marker).map_err(error::store) {
        return Report::fail(mutation.cmd, error);
    }

    let mut entry = entry(env, &mutation, &prepared);
    if let Err(error) = stage::stage(env, &entry) {
        return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error);
    }
    if let Err(error) = projection::materialize(env, &prepared.projection, &prepared.old_tree, &prepared.new_tree) {
        return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error);
    }

    let passed = match gate::gate(env, mutation.strict) {
        Ok(passed) => passed,
        Err(error) => return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error),
    };
    let ms = started.elapsed().as_millis() as u64;

    if !passed {
        return outcome::reject(env, &prepared.old_tree, &mut entry, mutation.cmd, ms);
    }
    if mutation.dry_run {
        return outcome::preview(env, &prepared.old_tree, &mut entry, mutation.cmd, ms);
    }

    entry.gate = "model".to_string();
    if let Err(error) = stage::commit(env, prepared.seq, &prepared.catalog, &entry) {
        return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error);
    }

    let mut report = Report::new(mutation.cmd);
    report.seq = Some(prepared.seq);
    report.changed = prepared.changed;
    report.lines = report.changed.iter().map(|change| change.id.clone()).collect();
    report.gate = Some(super::report::Gate {
        tier: "model".to_string(),
        new: 0,
        fixed: 0,
        existing: 0,
        verdict: "ok".to_string(),
        ms,
    });
    report.notes = notes;
    report
}

fn prepare(env: &super::Env<'_>, mutation: &Mutation, notes: &mut Vec<String>) -> Result<Prepared, error::Error> {
    load::ensure_crate(env)?;
    let catalog = load::catalog(env)?;
    let policy = load::policy(env)?;
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
        changed,
    })
}

fn entry(env: &super::Env<'_>, mutation: &Mutation, prepared: &Prepared) -> Entry {
    Entry {
        seq: prepared.seq,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op: mutation.op,
        target: mutation.target.clone(),
        from: prepared.old_digest,
        to: prepared.new_digest,
        gate: "pending".to_string(),
        msg: mutation.message.clone(),
    }
}

