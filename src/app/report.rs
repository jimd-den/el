// What one command did, as plain data. Both renderers read this and nothing else,
// which is why `--json` cannot drift from the human output: they are two views of
// one value, not two paths through the code.
//
// The fields that appear in `COMMANDS.md` §5 are the contract (`el`, `cmd`, `ok`,
// `error.*`, `seq`, `changed[].*`, `gate.*`); `lines` and `notes` are the human
// view of the same facts and carry no promise.

use super::Exit;

/// The envelope version. Bumped only when a stable field changes meaning.
pub const ENVELOPE: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    /// `add`, `edit`, `rm`, `mv`, `rename` — the operation, in the model's words.
    pub op: &'static str,
    pub id: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDiagnostic {
    pub code: String,
    pub level: String,
    /// The element the diagnostic is attributed to, if the span mapped.
    pub el: Option<String>,
    /// The owning item of an attributed statement, if any.
    pub owner: Option<String>,
    pub path: String,
    pub line: u32,
    pub col: u32,
    pub message: String,
    /// False when the diagnostic fell back to file/line attribution.
    pub exact: bool,
    /// Exactly one new diagnostic is marked on a rejection: the one attributed to
    /// an element the change touched, or the first exact one when none was.
    pub first_actionable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub tier: String,
    pub new: usize,
    pub fixed: usize,
    pub existing: usize,
    /// `ok`, `rejected` or `preview`.
    pub verdict: String,
    pub ms: u64,
    /// The new diagnostics, empty on a pass. Omitted from `--json` when empty.
    pub diagnostics: Vec<GateDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// A namespaced, stable code: `model.ceiling`, `env.drift`, `conflict.locked`, …
    pub code: &'static str,
    pub exit: Exit,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub cmd: &'static str,
    pub seq: Option<u64>,
    pub changed: Vec<Changed>,
    pub gate: Option<Gate>,
    /// Lines a human wants and a script must not parse (`COMMANDS.md` §6).
    pub lines: Vec<String>,
    /// Prose about the change itself: warnings, what was chosen, what was skipped.
    pub notes: Vec<String>,
    /// Verb-specific machine data that has no place in the stable envelope. A
    /// `log` puts its records here; a mutation leaves it `None` and uses
    /// `changed`/`gate`.
    pub payload: Option<crate::core::text::json::Json>,
    pub error: Option<Error>,
}

/// The next command a refusal names. One line per published code, so a failure
/// that leaves the user with nowhere to go is a test failure rather than a habit.
/// The fallback is the page that names every other command.
pub const HINTS: &[(&str, &str)] = &[
    ("usage.unknown_verb", "el help"),
    ("usage.unknown_flag", "el help"),
    ("usage.unknown_kind", "el help"),
    ("usage.missing_verb", "el help"),
    ("usage.missing_argument", "el help"),
    ("usage.missing_value", "el help"),
    ("usage.repeated_flag", "el help"),
    ("usage.bad_value", "el help"),
    ("usage.unexpected_argument", "el help"),
    ("usage.unsupported", "el help"),
    ("usage.no_change", "el help"),
    ("usage.bad_batch", "el help"),
    ("usage.bad_script", "el help"),
    ("usage.exists", "el init"),
    ("model.ceiling", "el ls"),
    ("model.duplicate_alias", "el ls"),
    ("model.dangling_ref", "el rm"),
    ("model.cross_body_move", "el tree"),
    ("model.unbalanced", "el help"),
    ("model.unknown_ref", "el ls"),
    ("model.bad_ident", "el help"),
    ("model.fn_budget", "el tree"),
    ("model.preamble", "el tree"),
    ("model.forbidden", "el help"),
    ("model.dep_denied", "el help"),
    ("model.bad_stmt", "el tree"),
    ("notfound.element", "el ls"),
    ("notfound.sequence", "el log"),
    ("gate.rejected", "el diff"),
    ("gate.would_reject", "el diff"),
    ("env.not_a_crate", "el new hello"),
    ("env.drift", "el undo"),
    ("env.bad_policy", "el doctor"),
    ("env.io", "el doctor"),
    ("env.corrupt", "el doctor"),
    ("env.missing_tool", "el help"),
    ("env.tool_failed", "el doctor"),
    ("env.timeout", "el doctor"),
    ("env.run_failed", "el log"),
    ("env.offline_miss", "el help"),
    ("conflict.locked", "el doctor"),
    ("ra.timeout", "el help"),
    ("internal.panic", "el doctor"),
];

/// The hint for a code. An unpublished code still gets a valid page rather than
/// an empty line; the tests above make the table the thing that grows.
pub fn hint_for(code: &str) -> &'static str {
    HINTS
        .iter()
        .find(|(known, _)| *known == code)
        .map(|(_, hint)| *hint)
        .unwrap_or("el help")
}

impl Error {
    /// The next command this refusal names.
    pub fn hint(&self) -> &'static str {
        hint_for(self.code)
    }
}

impl Report {
    /// A report for a command that has not yet had a chance to fail.
    pub fn new(cmd: &'static str) -> Self {
        Self {
            cmd,
            seq: None,
            changed: Vec::new(),
            gate: None,
            lines: Vec::new(),
            notes: Vec::new(),
            payload: None,
            error: None,
        }
    }

    /// The exit code this report implies: its error's, or success.
    pub fn exit(&self) -> Exit {
        self.error.as_ref().map_or(Exit::Ok, |error| error.exit)
    }

    pub fn fail(cmd: &'static str, error: Error) -> Self {
        Self {
            error: Some(error),
            ..Self::new(cmd)
        }
    }

    /// Record a refusal without losing what the command had already done.
    pub fn with_error(mut self, code: &'static str, exit: Exit, message: impl Into<String>) -> Self {
        self.error = Some(Error {
            code,
            exit,
            message: message.into(),
        });
        self
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
