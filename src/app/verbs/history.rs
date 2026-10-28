// Revert appends an inverse change: a head move plus a full tree
// materialize, not a reverse diff, so it cannot half-apply (FR-29).
// Undo and redo live in history_moves: since ADR-0032 they append
// entries too, instead of silently moving head. The helpers below
// serve both modules.

use super::super::{error, load, projection, transaction, Env, Exit, Report};
use crate::core::history::entry::{Entry, Op};
use crate::core::history::object;
use crate::core::history::tree::Tree;

pub fn revert(env: &Env<'_>, seq: u64) -> Report {
    let target = match entry_at(env, seq) {
        Ok(Some(entry)) => entry,
        Ok(None) => return Report::fail("revert", missing(&format!("sequence {seq} does not exist"))),
        Err(error) => return Report::fail("revert", error),
    };
    let (head, old_tree) = match state(env, "revert") {
        Ok(state) => state,
        Err(error) => return Report::fail("revert", error),
    };
    let target_tree = match env.journal.get_tree(&target.to).map_err(error::store) {
        Ok(tree) => tree,
        Err(error) => return Report::fail("revert", error),
    };
    if let Err(error) = env.journal.truncate_future(head).map_err(error::store) {
        return Report::fail("revert", error);
    }
    let new_seq = head + 1;
    let marker = format!("{} {} {}", new_seq, object::show(&target.to), env.clock.now_unix());
    if let Err(error) = env.journal.set_inflight(&marker).map_err(error::store) {
        return Report::fail("revert", error);
    }
    let old_digest = match env.journal.put_tree(&old_tree).map_err(error::store) {
        Ok(digest) => digest,
        Err(error) => return abort(env, &old_tree, "revert", error),
    };
    if let Err(error) = projection::restore(env, &target_tree) {
        return abort(env, &old_tree, "revert", error);
    }
    let entry = Entry {
        seq: new_seq,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op: Op::Revert,
        target: seq.to_string(),
        from: old_digest,
        to: target.to,
        gate: "model".to_string(),
        msg: format!("revert to {seq}"),
    };
    if let Err(error) = env.journal.put_entry(&entry).map_err(error::store) {
        return abort(env, &old_tree, "revert", error);
    }
    if let Err(error) = env.journal.set_head(new_seq).map_err(error::store) {
        return abort(env, &old_tree, "revert", error);
    }
    if let Err(error) = set_catalog_at(env, seq) {
        return abort(env, &old_tree, "revert", error);
    }
    let catalog = match env.journal.catalog_at(seq).map_err(error::store) {
        Ok(Some(text)) => text,
        _ => return abort(env, &old_tree, "revert", error::corrupt("revert target has no catalog")),
    };
    if let Err(error) = env.journal.put_catalog(new_seq, &catalog).map_err(error::store) {
        return abort(env, &old_tree, "revert", error);
    }
    let _ = env.journal.clear_inflight();
    let mut report = done("revert", new_seq, &format!("reverted to {seq}"));
    report.payload = Some(super::read::entry_payload(&entry));
    report
}

pub(crate) fn state(env: &Env<'_>, cmd: &'static str) -> Result<(u64, Tree), error::Error> {
    load::ensure_crate(env)?;
    let notes = transaction::recover(env)?;
    let _ = notes;
    let head = env
        .journal
        .head()
        .map_err(error::store)?
        .ok_or_else(|| missing("the crate has no history yet"))?;
    let (_, tree) = load::head_tree(env)?;
    let _ = cmd;
    Ok((head, tree))
}

pub(crate) fn entry_at(env: &Env<'_>, seq: u64) -> Result<Option<Entry>, error::Error> {
    if let Some(entry) = env.journal.entry(seq).map_err(error::store)? {
        return Ok(Some(entry));
    }
    Ok(env
        .journal
        .attic_entries()
        .map_err(error::store)?
        .into_iter()
        .find(|entry| entry.seq == seq))
}

pub(crate) fn restore_entry(env: &Env<'_>, entry: &Entry) -> Result<(), error::Error> {
    let tree = env.journal.get_tree(&entry.to).map_err(error::store)?;
    projection::restore(env, &tree)?;
    env.journal.set_head(entry.seq).map_err(error::store)?;
    set_catalog_at(env, entry.seq)
}

pub(crate) fn set_catalog_at(env: &Env<'_>, seq: u64) -> Result<(), error::Error> {
    let text = env
        .journal
        .catalog_at(seq)
        .map_err(error::store)?
        .ok_or_else(|| error::corrupt(format!("catalog snapshot for seq {seq} is missing")))?;
    env.journal.set_catalog(&text).map_err(error::store)
}

pub(crate) fn done(cmd: &'static str, seq: u64, line: &str) -> Report {
    let mut report = Report::new(cmd);
    report.seq = Some(seq);
    report.lines.push(line.to_string());
    report
}

fn abort(env: &Env<'_>, old_tree: &Tree, cmd: &'static str, cause: error::Error) -> Report {
    let _ = projection::restore(env, old_tree);
    let _ = env.journal.clear_inflight();
    Report::fail(cmd, cause)
}

pub(crate) fn missing(message: &str) -> error::Error {
    error::Error {
        code: "notfound.sequence",
        exit: Exit::NotFound,
        message: message.to_string(),
    }
}
