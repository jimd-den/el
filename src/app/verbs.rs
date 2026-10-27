// The verbs that are a thin shell over `change` and `transaction`, plus the two
// reads (`log`, `version`) that never write. Their only decision is what the
// journal's `op`, `target` and message are, because that is what a human reads
// back six months later.

use super::{change, load};
use super::report::Report;
use super::transaction::Mutation;
use super::Env;
use crate::cli::request::{Add, Edit, Globals, Mv};
use crate::core::history::entry::Op;

/// Whether a return-type note speaks on this report. It speaks when
/// the gate rejected with a type mismatch, or when the compiler was
/// skipped (`--fast`). On a checked success it would only fire for
/// `()`-valued tails, which is noise, so it stays silent there.
/// (Test bodies are typechecked by the gate too — a test with a
/// non-`()` tail is E0308-rejected at add time, not silently kept.)
fn speak_note(report: &Report, fast: bool) -> bool {
    match &report.error {
        None => fast,
        Some(error) if error.code == "gate.rejected" || error.code == "gate.would_reject" => {
            report
                .gate
                .as_ref()
                .map(|gate| gate.diagnostics.iter().any(|diag| diag.code == "E0308"))
                .unwrap_or(false)
        }
        _ => false,
    }
}

mod apply;
mod batch;
mod check;
mod diff;
mod doctor;
mod history;
mod inspect;
mod read;

pub use apply::apply;
pub use batch::batch;
pub use check::check;
pub use diff::diff;
pub use doctor::doctor;
pub use history::{redo, revert, undo};
pub use inspect::{explain, find, ls, show, tree};
pub use read::{log, run, version};

/// The one page `el skill`, `el help` and `--help` print. It reads the CLI's own
/// tables, so the page cannot list a verb the parser refuses.
pub fn guide(cmd: &'static str) -> Report {
    let mut report = Report::new(cmd);
    report.lines = crate::cli::table::guide().lines().map(str::to_string).collect();
    report
}

pub fn add(env: &Env<'_>, globals: &Globals, add: Add, stdin: &str) -> Report {
    let split = match load::policy(env) {
        Ok(policy) => policy.split,
        Err(error) => return Report::fail("add", error),
    };
    let change = match change::add(env, add.clone(), stdin, split) {
        Ok(change) => change,
        Err(error) => return Report::fail("add", error),
    };
    let note = match &change {
        crate::core::model::change::Change::AddElement { body, .. } => {
            change::return_note(&add, body).then_some(change::RETURN_NOTE.to_string())
        }
        _ => None,
    };
    let mut report = mutate(
        env,
        globals,
        Mutation {
            cmd: "add",
            op: Op::Add,
            target: format!("{} {}", add.kind, add.name),
            message: format!("add {} {}", add.kind, add.name),
            changes: vec![change],
            origins: Vec::new(),
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    );
    if speak_note(&report, globals.fast) && let Some(note) = note {
        report.notes.push(note);
    }
    report
}

pub fn edit(env: &Env<'_>, globals: &Globals, edit: Edit, stdin: &str) -> Report {
    let split = match load::policy(env) {
        Ok(policy) => policy.split,
        Err(error) => return Report::fail("edit", error),
    };
    let change = match change::edit(env, edit.clone(), stdin, split) {
        Ok(change) => change,
        Err(error) => return Report::fail("edit", error),
    };
    mutate(
        env,
        globals,
        Mutation {
            cmd: "edit",
            op: Op::Edit,
            target: edit.id.clone(),
            message: format!("edit {}", edit.id),
            changes: vec![change],
            origins: Vec::new(),
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    )
}

pub fn remove(env: &Env<'_>, globals: &Globals, ids: &[String]) -> Report {
    let changes = match change::remove(ids) {
        Ok(changes) => changes,
        Err(error) => return Report::fail("rm", error),
    };
    mutate(
        env,
        globals,
        Mutation {
            cmd: "rm",
            op: Op::Rm,
            target: ids.join(" "),
            message: format!("rm {}", ids.join(" ")),
            changes,
            origins: Vec::new(),
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    )
}

pub fn move_(env: &Env<'_>, globals: &Globals, mv: Mv) -> Report {
    let change = match change::move_(mv.clone()) {
        Ok(change) => change,
        Err(error) => return Report::fail("mv", error),
    };
    mutate(
        env,
        globals,
        Mutation {
            cmd: "mv",
            op: Op::Mv,
            target: mv.subject.clone(),
            message: format!("mv {} {}", mv.subject, mv.module.clone().unwrap_or_default()),
            changes: vec![change],
            origins: Vec::new(),
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    )
}

fn mutate(env: &Env<'_>, _globals: &Globals, mutation: Mutation) -> Report {
    let cmd = mutation.cmd;
    match super::transaction::recover(env) {
        Ok(_) => {}
        Err(error) => return Report::fail(cmd, error),
    }
    super::transaction::apply(env, mutation)
}

