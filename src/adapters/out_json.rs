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

use crate::app::report::{Changed, Gate, Report, ENVELOPE};
use crate::core::text::json::{self, Json};

fn text(value: &str) -> Json {
    Json::Str(value.to_string())
}

fn count(value: usize) -> Json {
    Json::Number(value.to_string())
}

/// One JSON object for `report`, newline-terminated so it is the last line of stdout.
pub fn render(report: &Report) -> String {
    let mut fields: Vec<(String, Json)> = Vec::with_capacity(8);
    let mut push = |key: &str, value: Json| fields.push((key.to_string(), value));

    push("el", text(ENVELOPE));
    push("cmd", text(report.cmd));
    push("ok", Json::Bool(report.error.is_none()));

    match &report.error {
        Some(error) => push(
            "error",
            Json::Object(vec![
                ("code".to_string(), text(error.code)),
                ("exit".to_string(), count(error.exit.code() as usize)),
                ("message".to_string(), text(&error.message)),
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

    let mut line = json::write(&Json::Object(fields));
    line.push('\n');
    line
}

fn changed(change: &Changed) -> Json {
    let mut fields = vec![
        ("op".to_string(), text(change.op)),
        ("id".to_string(), text(&change.id)),
    ];
    if let Some(alias) = &change.alias {
        fields.push(("alias".to_string(), text(alias)));
    }
    Json::Object(fields)
}

fn gate_json(gate: &Gate) -> Json {
    Json::Object(vec![
        ("tier".to_string(), text(&gate.tier)),
        ("new".to_string(), count(gate.new)),
        ("fixed".to_string(), count(gate.fixed)),
        ("existing".to_string(), count(gate.existing)),
        ("verdict".to_string(), text(&gate.verdict)),
        ("ms".to_string(), count(gate.ms as usize)),
    ])
}

#[cfg(test)]
#[path = "out_json_tests.rs"]
mod out_json_tests;
