// Journaled undo and redo (ADR-0032). Every head state is a journaled
// sequence: undo appends one entry per sequence stepped back, and redo
// appends an entry reversing the latest un-redone undo in the trailing
// undo/redo run. A new mutation after undos breaks the run, so redo
// still ends there; the undone work stays revertible. For crates
// written by older binaries with head behind the tip, redo keeps the
// old head-move path when a head + 1 entry exists.

use super::super::{error, projection, Env, Report};
use super::history;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::object;

pub fn undo(env: &Env<'_>, n: Option<usize>) -> Report {
    let n = n.unwrap_or(1) as u64;
    let (head, _) = match history::state(env, "undo") {
        Ok(state) => state,
        Err(error) => return Report::fail("undo", error),
    };
    if n == 0 {
        return history::done("undo", head, "nothing to undo");
    }
    if head.checked_sub(n).filter(|seq| *seq >= 1).is_none() {
        return Report::fail("undo", history::missing("there is no sequence that far back"));
    }
    let mut report = Report::new("undo");
    let mut restored = head;
    let mut current = head;
    for _ in 0..n {
        let undone = restored;
        let target = restored - 1;
        let undone_entry = match history::entry_at(env, undone) {
            Ok(Some(entry)) => entry,
            Ok(None) => return Report::fail("undo", history::missing("the undone sequence is gone")),
            Err(error) => return Report::fail("undo", error),
        };
        let msg = format!("undo {}", undone_entry.msg);
        match append_move(env, Op::Undo, target, &msg) {
            Ok(entry) => {
                report.lines.push(format!(
                    "undid {} (head {} -> {})",
                    undone_entry.msg, current, entry.seq
                ));
                report.payload = Some(super::read::entry_payload(&entry));
                current = entry.seq;
                restored = target;
            }
            Err(error) => return Report::fail("undo", error),
        }
    }
    report.seq = Some(current);
    report.lines.push("redo with `el redo`".to_string());
    report
}

pub fn redo(env: &Env<'_>) -> Report {
    let (head, _) = match history::state(env, "redo") {
        Ok(state) => state,
        Err(error) => return Report::fail("redo", error),
    };
    if let Some(target) = redoable_target(env) {
        let original = match history::entry_at(env, target) {
            Ok(Some(entry)) => entry,
            Ok(None) => return Report::fail("redo", history::missing("the undone sequence is gone")),
            Err(error) => return Report::fail("redo", error),
        };
        let msg = format!("redo {}", original.msg);
        return match append_move(env, Op::Redo, target, &msg) {
            Ok(entry) => {
                let line = format!("redid {} (head {} -> {})", original.msg, head, entry.seq);
                let mut report = history::done("redo", entry.seq, &line);
                report.payload = Some(super::read::entry_payload(&entry));
                report
            }
            Err(error) => Report::fail("redo", error),
        };
    }
    let target = head + 1;
    let entry = match env.journal.entry(target).map_err(error::store) {
        Ok(Some(entry)) => entry,
        Ok(None) => return Report::fail("redo", history::missing("nothing to redo")),
        Err(error) => return Report::fail("redo", error),
    };
    match history::restore_entry(env, &entry) {
        Ok(()) => {
            let line = format!("redid {} (head {} -> {})", entry.msg, head, target);
            let mut report = history::done("redo", target, &line);
            report.payload = Some(super::read::entry_payload(&entry));
            report
        }
        Err(error) => Report::fail("redo", error),
    }
}

/// The sequence a redo would restore, if the trailing undo/redo run
/// holds an undo no later redo in the run has reversed. An undo entry
/// restores its target and undid target + 1; a redo entry with that
/// target as its own target is the reversal.
fn redoable_target(env: &Env<'_>) -> Option<u64> {
    let mut entries = env.journal.entries().ok()?;
    entries.sort_by_key(|entry| entry.seq);
    let mut run: Vec<&Entry> = Vec::new();
    for entry in entries.iter().rev() {
        match entry.op {
            Op::Undo | Op::Redo => run.push(entry),
            _ => break,
        }
    }
    let redone: Vec<String> = run
        .iter()
        .filter(|entry| entry.op == Op::Redo)
        .map(|entry| entry.target.clone())
        .collect();
    for entry in &run {
        if entry.op != Op::Undo {
            continue;
        }
        let Ok(restored) = entry.target.parse::<u64>() else {
            continue;
        };
        let undone = restored + 1;
        if !redone.contains(&undone.to_string()) {
            return Some(undone);
        }
    }
    None
}

/// Append one history-move entry restoring `target_seq`'s tree and
/// catalog, following revert's skeleton: truncate the future, mark
/// inflight, restore the projection, append, advance head, store the
/// catalog, clear the marker. A failure restores the old tree.
fn append_move(env: &Env<'_>, op: Op, target_seq: u64, msg: &str) -> Result<Entry, error::Error> {
    let (head, old_tree) = history::state(env, op.as_str())?;
    let target_entry = history::entry_at(env, target_seq)?
        .ok_or_else(|| history::missing(&format!("sequence {target_seq} does not exist")))?;
    let target_tree = env.journal.get_tree(&target_entry.to).map_err(error::store)?;
    env.journal.truncate_future(head).map_err(error::store)?;
    let new_seq = head + 1;
    let marker = format!("{} {} {}", new_seq, object::show(&target_entry.to), env.clock.now_unix());
    env.journal.set_inflight(&marker).map_err(error::store)?;
    let old_digest = match env.journal.put_tree(&old_tree).map_err(error::store) {
        Ok(digest) => digest,
        Err(error) => return abort(env, &old_tree, error),
    };
    if let Err(error) = projection::restore(env, &target_tree) {
        return abort(env, &old_tree, error);
    }
    let entry = Entry {
        seq: new_seq,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op,
        target: target_seq.to_string(),
        from: old_digest,
        to: target_entry.to,
        gate: "model".to_string(),
        msg: msg.to_string(),
    };
    if let Err(error) = env.journal.put_entry(&entry).map_err(error::store) {
        return abort(env, &old_tree, error);
    }
    if let Err(error) = env.journal.set_head(new_seq).map_err(error::store) {
        return abort(env, &old_tree, error);
    }
    if let Err(error) = history::set_catalog_at(env, target_seq) {
        return abort(env, &old_tree, error);
    }
    let catalog = match env.journal.catalog_at(target_seq).map_err(error::store)? {
        Some(text) => text,
        None => {
            return abort(
                env,
                &old_tree,
                error::corrupt("move target has no catalog"),
            );
        }
    };
    if let Err(error) = env.journal.put_catalog(new_seq, &catalog).map_err(error::store) {
        return abort(env, &old_tree, error);
    }
    let _ = env.journal.clear_inflight();
    Ok(entry)
}

fn abort(env: &Env<'_>, old_tree: &crate::core::history::tree::Tree, cause: error::Error) -> Result<Entry, error::Error> {
    let _ = projection::restore(env, old_tree);
    let _ = env.journal.clear_inflight();
    Err(cause)
}
