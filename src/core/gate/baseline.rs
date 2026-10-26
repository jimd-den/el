// The baseline file: `.el/diags/<treeDigest>`, one line per diagnostic,
// tagged with its tier so one file holds every tier's set for that tree.
// A file that exists records a set — even an empty one, for a clean tree —
// while a missing file means "never gated at this tier", which the gate
// answers by computing the previous tree once, never by reporting a flood
// (GATE.md §4.2). Malformed lines are skipped, not fatal: a baseline is a
// cache, and a cache must fail soft.

use super::diagnostics::{Diagnostic, Level};
use crate::core::model::id::ElId;

/// One line: `el1` tier code level el owner exact path line col message.
/// The message is sanitized on write, so a line never contains a raw tab or
/// newline; the reader can therefore split on tabs without ambiguity.
pub fn encode(tier: &str, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diag in diagnostics {
        out.push_str(&format!(
            "el1\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tier,
            diag.code,
            diag.level.as_str(),
            diag.el.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string()),
            diag.owner
                .map(|id| id.to_string())
                .unwrap_or_else(|| "-".to_string()),
            if diag.exact { "1" } else { "0" },
            diag.path,
            diag.line,
            diag.col,
            diag.message.replace(['\t', '\n'], " "),
        ));
    }
    out
}

pub fn decode(text: &str, tier: &str) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 11 || parts[0] != "el1" || parts[1] != tier {
            continue;
        }
        let Some(level) = Level::parse(parts[3]) else {
            continue;
        };
        let Some(el) = parse_id(parts[4]) else {
            continue;
        };
        let Some(owner) = parse_id(parts[5]) else {
            continue;
        };
        let exact = match parts[6] {
            "1" => true,
            "0" => false,
            _ => continue,
        };
        let Ok(line_no) = parts[8].parse::<u32>() else {
            continue;
        };
        let Ok(col) = parts[9].parse::<u32>() else {
            continue;
        };
        found.push(Diagnostic {
            code: parts[2].to_string(),
            level,
            el,
            owner,
            path: parts[7].to_string(),
            line: line_no,
            col,
            message: parts[10].to_string(),
            exact,
        });
    }
    found
}

/// `"-"` means absent; anything else must be an `el-N` id. This parses the
/// digits itself rather than `ElId::parse`, which accepts exactly four
/// digits and would drop every element past `el-9999` from a baseline.
fn parse_id(text: &str) -> Option<Option<ElId>> {
    if text == "-" {
        return Some(None);
    }
    let digits = text.strip_prefix("el-")?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().map(|n| Some(ElId(n)))
}

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod baseline_tests;
