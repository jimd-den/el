// Turning script lines into changes. The verbs are the same ones the command line
// uses, and every failure names the line it came from, so an agent can repair the
// script without guessing which item refused.

use super::super::super::change;
use super::super::super::error::{self, Error};
use super::super::super::{Env, Exit};
use crate::cli::request::{Command, Globals, Source};
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;

pub(super) fn build(
    env: &Env<'_>,
    lines: &[Vec<String>],
    catalog: &Catalog,
    split: crate::core::plan::policy::Split,
) -> Result<Vec<(usize, crate::core::model::change::Change)>, Error> {
    let mut changes = Vec::new();
    for (index, argv) in lines.iter().enumerate() {
        let number = index + 1;
        let parsed = crate::cli::parse(argv).map_err(|usage| {
            line_error(number, usage_error(usage.code, usage.message))
        })?;
        if parsed.globals != Globals::default() {
            return Err(line_error(
                number,
                usage("global flags belong to apply, not to a script line"),
            ));
        }
        check_ids(&parsed.command, catalog, number)?;
        let built = one(env, parsed.command, number, split)?;
        changes.extend(built.into_iter().map(|change| (number, change)));
    }
    Ok(changes)
}

fn one(
    env: &Env<'_>,
    command: Command,
    number: usize,
    split: crate::core::plan::policy::Split,
) -> Result<Vec<crate::core::model::change::Change>, Error> {
    match command {
        Command::Add(add) => {
            no_stdin(&add.body, number)?;
            no_stdin(&add.preamble, number)?;
            no_stdin(&add.doc, number)?;
            change::add(env, add, "", split)
                .map(|change| vec![change])
                .map_err(|error| line_error(number, error))
        }
        Command::Edit(edit) => {
            no_stdin(&edit.body, number)?;
            no_stdin(&edit.doc, number)?;
            change::edit(env, edit, "", split)
                .map(|change| vec![change])
                .map_err(|error| line_error(number, error))
        }
        Command::Mv(mv) => change::move_(mv)
            .map(|change| vec![change])
            .map_err(|error| line_error(number, error)),
        Command::Rm { ids, cascade } => {
            if cascade {
                return Err(line_error(number, error::unsupported("apply", "rm --cascade")));
            }
            change::remove(&ids).map_err(|error| line_error(number, error))
        }
        other => Err(line_error(
            number,
            usage(&format!("{} is not a change a script can make", other.name())),
        )),
    }
}

fn check_ids(command: &Command, catalog: &Catalog, number: usize) -> Result<(), Error> {
    for text in ids(command) {
        let Some(id) = ElId::parse(text) else {
            return Err(line_error(
                number,
                usage(&format!("{text:?} is not an element id")),
            ));
        };
        if catalog.find(id).is_none() && catalog.find_stmt(id).is_none() {
            return Err(line_error(
                number,
                Error {
                    code: "notfound.element",
                    exit: Exit::NotFound,
                    message: format!("{id} does not exist when the script starts"),
                },
            ));
        }
    }
    Ok(())
}

fn ids(command: &Command) -> Vec<&String> {
    match command {
        Command::Add(add) => {
            let mut ids = Vec::new();
            if add.kind == crate::core::model::kind::Kind::Stmt {
                ids.push(&add.name);
            }
            ids.extend(add.after.iter());
            ids.extend(add.before.iter());
            ids
        }
        Command::Edit(edit) => vec![&edit.id],
        Command::Mv(mv) => {
            let mut ids = vec![&mv.subject];
            ids.extend(mv.after.iter());
            ids
        }
        Command::Rm { ids, .. } => ids.iter().collect(),
        _ => Vec::new(),
    }
}

fn no_stdin(source: &Option<Source>, number: usize) -> Result<(), Error> {
    if matches!(source, Some(Source::Stdin)) {
        return Err(line_error(
            number,
            usage("a script cannot read stdin; use --body <file>"),
        ));
    }
    Ok(())
}

pub(super) fn line_error(line: usize, mut error: Error) -> Error {
    error.message = format!("apply line {line}: {}", error.message);
    error
}

pub(super) fn usage(message: &str) -> Error {
    usage_error("usage.bad_script", message)
}

fn usage_error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        exit: Exit::Usage,
        message: message.into(),
    }
}
