// Parsing for the read verbs, kept out of `parse` so the parent stays a short
// dispatch table. It owns the one rule `ls` adds: `--kind` is a kind token, and an
// unknown token names the ones that exist.

use super::args::{Args, Usage};
use super::parse::count;
use super::request::{Command, Find, Ls, Show, Tree};
use crate::core::model::kind::Kind;

pub(super) fn command(verb: &str, args: &Args) -> Result<Option<Command>, Usage> {
    let command = match verb {
        "ls" => Command::Ls(Ls {
            module: args.value("--module")?,
            kind: kind(args.value("--kind")?)?,
            opaque: args.has("--opaque"),
        }),
        "discover" => {
            let query = match args.value("--query")? {
                Some(query) => query,
                None => {
                    return Err(Usage::new(
                        "usage.missing_argument",
                        "discover needs --query",
                    ))
                }
            };
            let limit = match count(args.value("--limit")?.as_deref())? {
                Some(limit) => limit,
                None => crate::app::verbs::DEFAULT_LIMIT,
            };
            let cursor = count(args.value("--cursor")?.as_deref())?.unwrap_or(0);
            Command::Discover { query, limit, cursor }
        }
        "read" => Command::Read {
            reference: args.required("read")?,
            rev: args.value("--rev")?,
            lines: lines(args.value("--lines")?)?,
        },
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
        "find" => Command::Find(Find {
            query: args.required("find")?,
        }),
        _ => return Ok(None),
    };
    Ok(Some(command))
}

/// `--lines start:end`, both ends numbers, exactly one colon.
fn lines(value: Option<String>) -> Result<Option<(usize, usize)>, Usage> {
    let Some(text) = value else {
        return Ok(None);
    };
    let mut parts = text.split(':');
    let start = parts.next().and_then(|part| part.parse().ok());
    let end = parts.next().and_then(|part| part.parse().ok());
    match (start, end, parts.next()) {
        (Some(start), Some(end), None) => Ok(Some((start, end))),
        _ => Err(Usage::new(
            "usage.bad_value",
            format!("--lines takes start:end, not {text:?}"),
        )),
    }
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
