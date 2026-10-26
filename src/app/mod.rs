// The application layer: the use cases, and the only place that knows the order a
// command's steps happen in. It reads ports and returns a `Report`; it never writes
// to a terminal, never names an adapter, and never exits the process.
//
// Shape: `run(parsed, &Env, stdin) -> Report`, where `Env` is the composition
// root's gift of ports. Every failure is a value (`Report::error`), so the process
// exit code is decided once, at the edge, from data a test can read.
//
// It deliberately does not decide *how* output looks: the human and JSON renderers
// are adapters reading the same `Report`, which is what keeps `--json` from being a
// second code path through every verb.

pub mod change;
pub mod create;
pub mod error;
pub mod exit;
pub mod inspect;
pub mod load;
pub mod projection;
pub mod report;
pub mod transaction;
pub mod verbs;

pub use exit::Exit;
pub use report::{Changed, Error, Gate, Report};

use crate::cli::request::{Command, Parsed};
use crate::ports::{Clock, Fs, Journal, Tools};

/// The ports and facts one command runs with. Constructed by `main.rs`, which is
/// the only place allowed to pick a concrete adapter.
pub struct Env<'a> {
    pub fs: &'a dyn Fs,
    pub journal: &'a dyn Journal,
    pub clock: &'a dyn Clock,
    pub tools: &'a dyn Tools,
    /// `$EL_ACTOR` or `human`; the journal records who asked.
    pub actor: String,
    /// The crate's package name, used by the renderer's manifest.
    pub crate_name: String,
    /// The root path, for the one message that has to name it.
    pub root: String,
    /// Seconds a gate subprocess may run before it is killed (`$EL_CARGO_TIMEOUT`).
    pub cargo_timeout: u64,
}

pub fn run(parsed: Parsed, env: &Env<'_>, stdin: &str) -> Report {
    let cmd = parsed.command.name();
    if parsed.globals.semantic {
        return Report::fail(cmd, error::unsupported(cmd, "--semantic (M10)"));
    }
    if parsed.globals.strict && parsed.globals.fast {
        return Report::fail(cmd, error::unsupported(cmd, "--strict with --fast"));
    }

    match &parsed.command {
        Command::New { path, lib, policy } => {
            create::new(env, &parsed.globals, path, *lib, *policy)
        }
        Command::Version => verbs::version(),
        Command::Init { .. } => Report::fail(
            "init",
            error::unsupported("init", "existing crate adoption (M12)"),
        ),
        Command::Add(add) => verbs::add(env, &parsed.globals, add.clone(), stdin),
        Command::Edit(edit) => verbs::edit(env, &parsed.globals, edit.clone(), stdin),
        Command::Rm { ids, cascade } => {
            if *cascade {
                Report::fail("rm", error::unsupported("rm", "--cascade"))
            } else {
                verbs::remove(env, &parsed.globals, ids)
            }
        }
        Command::Mv(mv) => verbs::move_(env, &parsed.globals, mv.clone()),
        Command::Run { args } => verbs::run(env, args),
        Command::Log { count, attic } => verbs::log(env, *count, *attic),
        Command::Diff { a, b } => verbs::diff(env, *a, *b),
        Command::Undo { n } => verbs::undo(env, *n),
        Command::Redo => verbs::redo(env),
        Command::Revert { seq } => verbs::revert(env, *seq),
        Command::Doctor { repair, rebuild } => verbs::doctor(env, *repair, *rebuild),
        Command::Ls(ls) => verbs::ls(env, ls.module.as_deref(), ls.kind, ls.opaque),
        Command::Tree(tree) => verbs::tree(env, tree.statements),
        Command::Show(show) => {
            verbs::show(env, &show.id, show.statements, show.source, show.history, show.terse)
        }
        Command::Find(find) => verbs::find(env, &find.query),
        Command::Batch => verbs::batch(env, &parsed.globals, stdin),
    }
}
