// A journal entry is one line that says what happened: which sequence number, when,
// by whom, what operation, on what, between which trees, with what verdict
// (`HISTORY.md` §4.1). `el log` is this file; undo is a head move between the trees it
// names.
//
// Shape: ten tab-separated fields, tag first so a future `el2` can be told apart from
// `el1` by a reader that was never taught about it. Free text is sanitized on the way
// in (`\t`, `\n`, `\r` become spaces), because a record that can contain a newline is
// a record that can forge the next one.
//
// It deliberately does not interpret `gate` or `target`. The verdict is the gate's
// vocabulary and the target is an id, a path or a dependency name; a codec that
// understood them would have to be updated every time one of them grew a case.

use super::object::{self, Digest};
use std::fmt;

/// What happened. One token per record, and the list is closed so a typo cannot
/// become a new operation nobody can search for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Init,
    Add,
    Edit,
    Rm,
    Mv,
    Dep,
    Policy,
    Adopt,
    Import,
    Revert,
    Repair,
    Apply,
    Undo,
    Redo,
}

impl Op {
    pub const ALL: [Op; 14] = [
        Op::Init,
        Op::Add,
        Op::Edit,
        Op::Rm,
        Op::Mv,
        Op::Dep,
        Op::Policy,
        Op::Adopt,
        Op::Import,
        Op::Revert,
        Op::Repair,
        Op::Apply,
        Op::Undo,
        Op::Redo,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Op::Init => "init",
            Op::Add => "add",
            Op::Edit => "edit",
            Op::Rm => "rm",
            Op::Mv => "mv",
            Op::Dep => "dep",
            Op::Policy => "policy",
            Op::Adopt => "adopt",
            Op::Import => "import",
            Op::Revert => "revert",
            Op::Repair => "repair",
            Op::Apply => "apply",
            Op::Undo => "undo",
            Op::Redo => "redo",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.as_str() == text)
    }
}

/// One journal record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub seq: u64,
    /// Unix seconds, from the injected clock — never read inside `core`.
    pub ts: i64,
    pub actor: String,
    pub op: Op,
    /// Element id, module path, dependency name, or `-` for a batch.
    pub target: String,
    pub from: Digest,
    pub to: Digest,
    /// The gate's verdict, verbatim: `ok:new/fixed/existing`, `rejected:<n>`,
    /// `preview:<n>`, `skip`, `model`, `ra`.
    pub gate: String,
    pub msg: String,
}

/// A record that cannot be read back as what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub field: &'static str,
    pub text: String,
}

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "env.corrupt: entry {} is {:?}", self.field, self.text)
    }
}

/// The most a message may be, in characters. Longer is truncated, because the record
/// is one line and the journal is not a log sink.
const MAX_MSG: usize = 200;

impl Entry {
    /// The record form: ten fields, one line, newline-terminated.
    pub fn encode(&self) -> String {
        format!(
            "el1\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            self.seq,
            self.ts,
            sanitize(&self.actor),
            self.op.as_str(),
            sanitize(&self.target),
            object::show(&self.from),
            object::show(&self.to),
            sanitize(&self.gate),
            truncate(&sanitize(&self.msg)),
        )
    }

    /// The inverse of `encode`.
    ///
    /// Contract: exactly ten fields and the `el1` tag; every rejection names the field
    /// that failed, so a corrupt entry says which part of it to distrust.
    pub fn decode(line: &str) -> Result<Self, Malformed> {
        let fields: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
        if fields.len() != 10 {
            return Err(Malformed {
                field: "field count",
                text: line.to_string(),
            });
        }
        if fields[0] != "el1" {
            return Err(Malformed {
                field: "tag",
                text: fields[0].to_string(),
            });
        }

        Ok(Self {
            seq: fields[1].parse().map_err(|_| malformed("seq", fields[1]))?,
            ts: fields[2].parse().map_err(|_| malformed("ts", fields[2]))?,
            actor: fields[3].to_string(),
            op: Op::parse(fields[4]).ok_or_else(|| malformed("op", fields[4]))?,
            target: fields[5].to_string(),
            from: object::parse(fields[6]).ok_or_else(|| malformed("from", fields[6]))?,
            to: object::parse(fields[7]).ok_or_else(|| malformed("to", fields[7]))?,
            gate: fields[8].to_string(),
            msg: fields[9].to_string(),
        })
    }
}

/// Free text made safe for a tab-separated, one-line record.
pub fn sanitize(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

fn truncate(text: &str) -> String {
    text.chars().take(MAX_MSG).collect()
}

fn malformed(field: &'static str, text: &str) -> Malformed {
    Malformed {
        field,
        text: text.to_string(),
    }
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
