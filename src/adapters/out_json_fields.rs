// The envelope's field builders: changed entries and gate diagnostics.
// They live apart from the renderer so the renderer stays under its
// ceiling; the renderer owns the field order, these own the shapes.

use super::out_json::{count, text};
use crate::app::report::{Changed, Gate, GateDiagnostic};
use crate::core::text::json::Json;

pub(super) fn changed(change: &Changed) -> Json {
    let mut fields = vec![
        ("op".to_string(), text(change.op)),
        ("id".to_string(), text(&change.id)),
    ];
    if let Some(alias) = &change.alias {
        fields.push(("alias".to_string(), text(alias)));
    }
    Json::Object(fields)
}

pub(super) fn gate_json(gate: &Gate) -> Json {
    let mut fields = vec![
        ("tier".to_string(), text(&gate.tier)),
        ("new".to_string(), count(gate.new)),
        ("fixed".to_string(), count(gate.fixed)),
        ("existing".to_string(), count(gate.existing)),
        ("verdict".to_string(), text(&gate.verdict)),
        ("ms".to_string(), count(gate.ms as usize)),
    ];
    if !gate.diagnostics.is_empty() {
        fields.push((
            "diagnostics".to_string(),
            Json::Array(gate.diagnostics.iter().map(diag_json).collect()),
        ));
    }
    Json::Object(fields)
}

pub(super) fn diag_json(diag: &GateDiagnostic) -> Json {
    let mut fields = vec![
        ("code".to_string(), text(&diag.code)),
        ("level".to_string(), text(&diag.level)),
    ];
    if let Some(el) = &diag.el {
        fields.push(("el".to_string(), text(el)));
    }
    if let Some(owner) = &diag.owner {
        fields.push(("owner".to_string(), text(owner)));
    }
    fields.push(("path".to_string(), text(&diag.path)));
    fields.push(("line".to_string(), count(diag.line as usize)));
    fields.push(("col".to_string(), count(diag.col as usize)));
    fields.push(("message".to_string(), text(&diag.message)));
    fields.push(("exact".to_string(), Json::Bool(diag.exact)));
    fields.push((
        "first_actionable".to_string(),
        Json::Bool(diag.first_actionable),
    ));
    Json::Object(fields)
}
