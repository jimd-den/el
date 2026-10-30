// The `--json` view: exactly one object on stdout, built from the same `Report` the
// human view reads. The field set is `COMMANDS.md` §5, which is a compatibility
// promise — so the keys are written once here, in the published order, and a golden
// test compares the bytes rather than the parsed value.
//
// Keys are emitted in insertion order, and the writer is canonical (no spaces), so
// the golden is one line a reviewer can read.
//
// It deliberately does not merge human prose into the envelope: `notes` is a list
// of strings and the human view owns the sentences. A field that only a human wants
// has no place in a machine contract.

use super::out_json_fields::{changed, gate_json};
use crate::app::report::{Report, ENVELOPE};
use crate::core::text::json::{self, Json};

pub(super) fn text(value: &str) -> Json {
    Json::Str(value.to_string())
}

pub(super) fn count(value: usize) -> Json {
    Json::Number(value.to_string())
}

/// Per-invocation id: process id plus wall-clock millis. It is the one
/// field that cannot be byte-pinned, so the golden tests shape-check it
/// and byte-compare everything else.
fn request_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("req-{}-{millis}", std::process::id())
}

/// One JSON object for `report`, newline-terminated so it is the last line of stdout.
pub fn render(report: &Report) -> String {
    let mut fields: Vec<(String, Json)> = Vec::with_capacity(8);
    let mut push = |key: &str, value: Json| fields.push((key.to_string(), value));

    push("el", text(ENVELOPE));
    push("schema_version", text("1"));
    push("request_id", text(&request_id()));
    push("cmd", text(report.cmd));
    push("action", text(report.cmd));
    push("ok", Json::Bool(report.error.is_none()));
    push("status", text(report.status()));

    match &report.error {
        Some(error) => push(
            "error",
            Json::Object(vec![
                ("code".to_string(), text(error.code)),
                ("exit".to_string(), count(error.exit.code() as usize)),
                ("message".to_string(), text(&error.message)),
                ("hint".to_string(), text(error.hint())),
            ]),
        ),
        None => {
            if let Some(seq) = report.seq {
                push("seq", Json::Number(seq.to_string()));
            }
            push("changed", Json::Array(report.changed.iter().map(changed).collect()));
        }
    }

    if let Some(gate) = &report.gate {
        push("gate", gate_json(gate));
    }
    if !report.notes.is_empty() {
        push("notes", Json::Array(report.notes.iter().map(|note| text(note)).collect()));
    }
    push("data", report.payload.clone().unwrap_or(Json::Object(vec![])));
    push(
        "refs",
        Json::Array(
            report
                .refs
                .iter()
                .map(|reference| {
                    Json::Object(vec![
                        ("ref".to_string(), text(&reference.reference)),
                        ("rev".to_string(), text(&reference.rev)),
                    ])
                })
                .collect(),
        ),
    );
    push("diagnostics", Json::Array(diagnostics(report)));
    push(
        "next",
        match &report.error {
            Some(error) => Json::Array(vec![text(error.hint())]),
            None => Json::Array(vec![]),
        },
    );
    if let Some(evidence) = &report.evidence {
        push("evidence", evidence.clone());
    }
    if let Some(pagination) = &report.pagination {
        push("pagination", pagination.clone());
    }
    if let Some(history) = &report.history {
        push("history", history.clone());
    }

    let mut line = json::write(&Json::Object(fields));
    line.push('\n');
    line
}

/// The envelope's diagnostics: the verb's typed list when it set one,
/// else one entry derived from the failure, else none.
fn diagnostics(report: &Report) -> Vec<Json> {
    if !report.diagnostics.is_empty() {
        return report
            .diagnostics
            .iter()
            .map(|diag| {
                let mut fields = vec![
                    ("code".to_string(), text(diag.code)),
                    ("message".to_string(), text(&diag.message)),
                    ("hint".to_string(), text(diag.hint)),
                ];
                if let Some(expected) = &diag.expected {
                    fields.push(("expected".to_string(), text(expected)));
                }
                if let Some(observed) = &diag.observed {
                    fields.push(("observed".to_string(), text(observed)));
                }
                if let Some(reference) = &diag.reference {
                    fields.push(("ref".to_string(), text(reference)));
                }
                Json::Object(fields)
            })
            .collect();
    }
    match &report.error {
        Some(error) => vec![Json::Object(vec![
            ("code".to_string(), text(error.code)),
            ("message".to_string(), text(&error.message)),
            ("hint".to_string(), text(error.hint())),
        ])],
        None => vec![],
    }
}

#[cfg(test)]
#[path = "out_json_tests.rs"]
mod out_json_tests;
