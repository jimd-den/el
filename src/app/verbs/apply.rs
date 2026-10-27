// `el apply`: a script of changes, one per line, all in one transaction. Each line
// is run through `cli::parse`, so a script and a command line share one grammar;
// every change is planned against the working catalog and all of them commit under
// one sequence — or none do.
//
// It deliberately does not invent ids: a script addresses what exists when it
// starts. An element added on line 1 has no id until planning, so editing it on
// line 2 is refused rather than guessed at.

use super::super::error::{self, Error};
use super::super::{load, transaction, Env, Exit, Report};
use super::mutate;
use crate::cli::request::Globals;

mod build;
use build::{line_error, usage};
use crate::core::history::entry::Op;

pub fn apply(env: &Env<'_>, globals: &Globals, file: Option<&str>, stdin: &str) -> Report {
    if let Err(error) = transaction::recover(env) {
        return Report::fail("apply", error);
    }
    let text = match source(env, file, stdin) {
        Ok(text) => text,
        Err(error) => return Report::fail("apply", error),
    };
    let lines = match crate::cli::script::parse(&text) {
        Ok(lines) => lines,
        Err(error) => return Report::fail("apply", line_error(error.line, usage(&error.message))),
    };
    if lines.is_empty() {
        return Report::fail("apply", usage("apply needs at least one change"));
    }
    let split = match load::policy(env) {
        Ok(policy) => policy.split,
        Err(error) => return Report::fail("apply", error),
    };
    let catalog = match load::catalog(env) {
        Ok(catalog) => catalog,
        Err(error) => return Report::fail("apply", error),
    };
    let built = match build::build(env, &lines, &catalog, split) {
        Ok(changes) => changes,
        Err(error) => return Report::fail("apply", error),
    };
    let count = built.len();
    let origins: Vec<usize> = built.iter().map(|(line, _)| *line).collect();
    let changes = built.into_iter().map(|(_, change)| change).collect();
    mutate(
        env,
        globals,
        transaction::Mutation {
            cmd: "apply",
            op: Op::Apply,
            target: format!("apply {count}"),
            message: format!("apply {count} changes"),
            changes,
            origins,
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    )
}

fn source(env: &Env<'_>, file: Option<&str>, stdin: &str) -> Result<String, Error> {
    match file {
        None | Some("-") => Ok(stdin.to_string()),
        Some(path) => {
            let bytes = env.fs.read_source(path).map_err(error::filesystem)?;
            let Some(bytes) = bytes else {
                return Err(Error {
                    code: "env.io",
                    exit: Exit::Env,
                    message: format!("{path} does not exist"),
                });
            };
            String::from_utf8(bytes)
                .map_err(|_| error::corrupt(format!("{path} is not UTF-8")))
        }
    }
}
