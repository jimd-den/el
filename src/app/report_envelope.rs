// The envelope's typed half: the revision-bound references, the typed
// diagnostics, the next-command table, and the status vocabulary. They
// live apart from `report` so the report file stays under its ceiling;
// `report` re-exports them, so every import path is unchanged.

use super::report::Report;

/// One revision-bound reference the response stands on: the addressed
/// thing and the digest of the content as read. `read` returns its
/// target; `discover` returns one per result; other verbs return none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeRef {
    pub reference: String,
    pub rev: String,
}

/// One typed diagnostic in the envelope's `diagnostics` list. `expected`,
/// `observed` and `reference` carry the stale-reference triple when they
/// apply; otherwise they are absent, not empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub hint: &'static str,
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub reference: Option<String>,
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
    ("usage.conflict", "el help"),
    ("usage.exists", "el ls"),
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
    ("model.opaque", "el tree"),
    ("notfound.element", "el ls"),
    ("notfound.sequence", "el log"),
    ("gate.rejected", "el diff"),
    ("gate.would_reject", "el diff"),
    ("gate.failed", "el check --json"),
    ("test.failed", "el test"),
    ("test.compile_failed", "el check"),
    ("EL_ENV_UNAVAILABLE", "el help"),
    ("EL_STALE_REF", "el ls"),
    ("EL_AMBIGUOUS_TARGET", "el ls"),
    ("EL_SCOPE_POLICY", "el help"),
    ("env.not_a_crate", "el new hello"),
    ("env.not_a_project", "el new hello"),
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


impl Report {
    /// The envelope's `status`: what the run established, in the
    /// plan's vocabulary. A refusal (validation, unknown target)
    /// is not a failure (verification ran and failed), and neither
    /// is a block (state or environment prevents the run).
    pub fn status(&self) -> &'static str {
        match &self.error {
            None => match self.cmd {
                "undo" | "redo" | "revert" => "restored",
                _ => "passed",
            },
            Some(error) => {
                let code = error.code;
                if code.starts_with("usage.")
                    || code.starts_with("notfound.")
                    || code.starts_with("model.")
                    || code == "EL_SCOPE_POLICY"
                    || code == "EL_AMBIGUOUS_TARGET"
                {
                    "refused"
                } else if code.starts_with("env.")
                    || code.starts_with("conflict.")
                    || code == "EL_ENV_UNAVAILABLE"
                    || code == "EL_STALE_REF"
                    || code == "EL_TRANSACTION_LOCKED"
                {
                    "blocked"
                } else {
                    "failed"
                }
            }
        }
    }
}
