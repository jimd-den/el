// History and doctor parsing, kept out of the main dispatch table. They share one
// value rule: a sequence number is a number, and a bad one is a usage error.

use super::args::{Args, Usage};
use super::request::Command;

pub(super) fn command(verb: &str, args: &Args) -> Result<Option<Command>, Usage> {
    let command = match verb {
        "diff" => Command::Diff {
            a: seq(args.words.get(1))?,
            b: seq(args.words.get(2))?,
        },
        "undo" => Command::Undo {
            n: count(args.words.get(1))?,
        },
        "redo" => Command::Redo,
        "revert" => Command::Revert {
            seq: seq(args.words.get(1))?.ok_or_else(|| {
                Usage::new("usage.missing_argument", "revert needs a sequence number")
            })?,
        },
        "doctor" => Command::Doctor {
            repair: args.has("--repair"),
            rebuild: args.has("--rebuild"),
        },
        _ => return Ok(None),
    };
    Ok(Some(command))
}

fn seq(value: Option<&String>) -> Result<Option<u64>, Usage> {
    match value {
        None => Ok(None),
        Some(text) => text.parse().map(Some).map_err(|_| {
            Usage::new("usage.bad_value", format!("a sequence number, not {text:?}"))
        }),
    }
}

fn count(value: Option<&String>) -> Result<Option<usize>, Usage> {
    match value {
        None => Ok(None),
        Some(text) => text.parse().map(Some).map_err(|_| {
            Usage::new("usage.bad_value", format!("a count, not {text:?}"))
        }),
    }
}
