// Parsing for the read verbs, kept out of `parse` so the parent stays a short
// dispatch table. It owns the one rule `ls` adds: `--kind` is a kind token, and an
// unknown token names the ones that exist.

use super::args::{Args, Usage};
use super::request::{Command, Ls, Show, Tree};
use crate::core::model::kind::Kind;

pub(super) fn command(verb: &str, args: &Args) -> Result<Option<Command>, Usage> {
    let command = match verb {
        "ls" => Command::Ls(Ls {
            module: args.value("--module")?,
            kind: kind(args.value("--kind")?)?,
            opaque: args.has("--opaque"),
        }),
        "tree" => Command::Tree(Tree {
            statements: args.has("--statements"),
        }),
        "show" => Command::Show(Show {
            id: args.required("show")?,
            statements: args.has("--statements"),
            source: args.has("--source"),
            history: args.has("--history"),
            terse: args.has("--terse"),
        }),
        _ => return Ok(None),
    };
    Ok(Some(command))
}

fn kind(value: Option<String>) -> Result<Option<Kind>, Usage> {
    match value {
        None => Ok(None),
        Some(token) => Kind::parse(&token).map(Some).ok_or_else(|| {
            Usage::new(
                "usage.unknown_kind",
                format!(
                    "--kind takes {}, not {:?}",
                    Kind::all().iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" "),
                    token
                ),
            )
        }),
    }
}
