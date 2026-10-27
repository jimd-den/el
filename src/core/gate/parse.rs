// `cargo check` JSON → diagnostics. One frame per line; only
// `reason == "compiler-message"` frames with level `error` or `warning` are
// diagnostics, so a cargo upgrade that adds frames, and rustc's own
// `failure-note` frames, cannot break the gate (GATE.md §6). Attribution is
// a caller-supplied closure over the span map: the parser owns the JSON
// shape, the caller owns what a line belongs to.

use super::diagnostics::{normalize, Diagnostic, Level};
use crate::core::model::id::ElId;
use crate::core::text::json::{self, Json};

/// Map a rendered `(path, 1-based line)` to the innermost element owning it
/// and, when that element is a statement, its owning item.
pub type Attribute<'a> = dyn Fn(&str, u32) -> Option<(ElId, Option<ElId>)> + 'a;

pub fn parse_cargo_json(stdout: &str, attribute: &Attribute<'_>) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for line in stdout.lines() {
        let Some(frame) = json::parse(line) else {
            continue;
        };
        if frame.get("reason").and_then(Json::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = frame.get("message") else {
            continue;
        };
        let Some(level) = message
            .get("level")
            .and_then(Json::as_str)
            .and_then(Level::parse)
        else {
            continue;
        };
        // A parse error carries no code (`"code": null`). `uncoded` is a value
        // rather than an empty string: `diagnostics[].code` is a published field,
        // and two different uncoded errors must not share one spelling.
        let code = message
            .get("code")
            .and_then(|code| code.get("code"))
            .and_then(Json::as_str)
            .unwrap_or("uncoded")
            .to_string();
        let raw = message
            .get("message")
            .and_then(Json::as_str)
            .unwrap_or("");
        let (path, line, col) = match primary_span(message) {
            Some((path, line, col)) => (strip_dot_slash(path), line, col),
            None => (String::new(), 0, 0),
        };
        // E0601 names a crate condition ("main function not found"), not an
        // element: its span points at arbitrary file content, so attributing
        // it makes its fingerprint flip whenever the span's owner changes
        // (ADR-0027). It stays unattributed and falls back to (code, path,
        // message), which is stable across adds until `fn main` lands.
        let attributed = (code != "E0601" && !path.is_empty())
            .then(|| attribute(&path, line))
            .flatten();
        let (el, owner, exact) = match attributed {
            Some((el, owner)) => (Some(el), owner, true),
            None => (None, None, false),
        };
        found.push(Diagnostic {
            code,
            level,
            el,
            owner,
            path,
            line,
            col,
            message: normalize(raw),
            exact,
        });
    }
    found
}

/// The span the diagnostic is about: the primary one when marked, else the
/// first. A diagnostic with no spans (a crate-level failure) has no position.
fn primary_span(message: &Json) -> Option<(String, u32, u32)> {
    let spans = message.get("spans")?.as_array()?;
    let span = spans
        .iter()
        .find(|span| matches!(span.get("is_primary"), Some(Json::Bool(true))))
        .or_else(|| spans.first())?;
    let path = span.get("file_name")?.as_str()?.to_string();
    let line = span.get("line_start")?.as_number()? as u32;
    let col = span.get("column_start")?.as_number()? as u32;
    Some((path, line, col))
}

fn strip_dot_slash(path: String) -> String {
    path.strip_prefix("./").unwrap_or(&path).to_string()
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
