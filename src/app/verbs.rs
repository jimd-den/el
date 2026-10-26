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

mod diff;
mod doctor;
mod history;
mod inspect;
mod read;

pub use diff::diff;
pub use doctor::doctor;
pub use history::{redo, revert, undo};
pub use inspect::{ls, show, tree};
pub use read::{log, run, version};

pub fn add(env: &Env<'_>, globals: &Globals, add: Add, stdin: &str) -> Report {
    let split = match load::policy(env) {
        Ok(policy) => policy.split,
        Err(error) => return Report::fail("add", error),
    };
    let change = match change::add(env, add.clone(), stdin, split) {
        Ok(change) => change,
        Err(error) => return Report::fail("add", error),
    };
    mutate(
        env,
        globals,
        Mutation {
            cmd: "add",
            op: Op::Add,
            target: format!("{} {}", add.kind, add.name),
            message: format!("add {} {}", add.kind, add.name),
            changes: vec![change],
            dry_run: globals.dry_run,
            strict: globals.strict,
        },
    )
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
            dry_run: globals.dry_run,
            strict: globals.strict,
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
            dry_run: globals.dry_run,
            strict: globals.strict,
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
            dry_run: globals.dry_run,
            strict: globals.strict,
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

