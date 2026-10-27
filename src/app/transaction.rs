// One mutating command, in the order `GATE.md` §3 fixes. The order is the crash
// contract: plan before any write, inflight before the first projection write, and
// head only after the gate speaks. A refusal is therefore a value with nothing to
// undo, and a crash leaves a marker the next invocation can act on.
//
// It deliberately does not parse a command line or render output. `verbs` builds
// the changes; `out_*` reads the report; this module decides only what is written
// and in what order.

use super::error::{self, Error};
use super::load;
use super::projection;
use super::report::Report;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::object;
use std::time::Instant;

pub(crate) use gate::{diagnostics, select};
pub(crate) use outcome::{diagnostic_lines, gate_from_diagnostics};

mod gate;
mod outcome;
mod prepare;
mod stage;

#[cfg(test)]
#[path = "transaction/support.rs"]
mod support;
#[cfg(test)]
#[path = "transaction_gate_tests.rs"]
mod transaction_gate_tests;
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
    /// One line number per change when the changes came from a script; empty for
    /// a command line. Used only to name the line in a plan refusal.
    pub origins: Vec<usize>,
    pub dry_run: bool,
    pub strict: bool,
    pub fast: bool,
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

pub fn apply(env: &super::Env<'_>, mutation: Mutation) -> Report {
    let mut notes = Vec::new();
    let prepared = match prepare::prepare(env, &mutation, &mut notes) {
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
    let written = match projection::materialize(env, &prepared.projection, &prepared.old_tree, &prepared.new_tree) {
        Ok(written) => written,
        Err(error) => return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error),
    };

    let evaluation = match gate::evaluate(gate::Input {
        env,
        tier: prepared.tier,
        catalog: &prepared.catalog,
        old_catalog: &prepared.old_catalog,
        projection: &prepared.projection,
        old_tree: &prepared.old_tree,
        new_tree: &prepared.new_tree,
        old_digest: prepared.old_digest,
    }) {
        Ok(evaluation) => evaluation,
        Err(error) => return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error),
    };
    let ms = started.elapsed().as_millis() as u64;
    let touched: Vec<crate::core::model::id::ElId> = prepared
        .changed
        .iter()
        .filter_map(|change| crate::core::model::id::ElId::parse(&change.id))
        .collect();
    let judged = outcome::GateVerdict {
        tier: prepared.tier,
        delta: &evaluation.delta,
        catalog: &prepared.catalog,
        touched: &touched,
    };

    if mutation.dry_run {
        return outcome::preview(
            env,
            &prepared.old_tree,
            &mut entry,
            mutation.cmd,
            ms,
            judged,
            gate::may_commit(prepared.tier, &evaluation.delta),
        );
    }
    if !gate::may_commit(prepared.tier, &evaluation.delta) {
        return outcome::reject(env, &prepared.old_tree, &mut entry, mutation.cmd, ms, judged);
    }

    entry.gate = prepared.tier.as_str().to_string();
    if prepared.tier != crate::core::gate::Tier::Model
        && let Err(error) = gate::write_baseline(
            env,
            prepared.new_digest,
            prepared.tier,
            &evaluation.current,
        )
    {
        return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error);
    }
    if let Err(error) = stage::commit(env, prepared.seq, &prepared.catalog, &entry) {
        return outcome::abort(env, &prepared.old_tree, &entry, mutation.cmd, error);
    }

    notes.push(written.message());
    let new_warnings = evaluation
        .delta
        .new
        .iter()
        .filter(|diag| diag.level == crate::core::gate::Level::Warning)
        .count();
    if new_warnings > 0 {
        notes.push(format!(
            "el: {} new warning{} ({} fixed, {} pre-existing)",
            new_warnings,
            if new_warnings == 1 { "" } else { "s" },
            evaluation.delta.fixed.len(),
            evaluation.delta.existing.len()
        ));
        notes.extend(outcome::diagnostic_lines(
            &prepared.catalog,
            &evaluation.delta.new,
            &touched,
        ));
    }

    let mut report = Report::new(mutation.cmd);
    report.seq = Some(prepared.seq);
    report.changed = prepared.changed;
    report.lines = report.changed.iter().map(|change| change.id.clone()).collect();
    report.gate = Some(super::report::Gate {
        tier: prepared.tier.as_str().to_string(),
        new: evaluation.delta.new.len(),
        fixed: evaluation.delta.fixed.len(),
        existing: evaluation.delta.existing.len(),
        verdict: "ok".to_string(),
        ms,
        diagnostics: vec![],
    });
    report.notes = notes;
    report
}

fn entry(env: &super::Env<'_>, mutation: &Mutation, prepared: &prepare::Prepared) -> Entry {
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

