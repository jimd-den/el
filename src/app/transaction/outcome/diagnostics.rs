// Presentation for a transaction that ends without committing: the diagnostic
// lines GATE §4.3 publishes, and the counts the envelope carries. Kept apart from
// `outcome` so the three endings there read as what they do — restore, journal,
// report — and the wording lives in one place instead of three.

use crate::app::report::{Gate, GateDiagnostic};
use crate::core::gate::{Delta, Diagnostic, Tier};
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// One header line and one message line per diagnostic, in the GATE §4.3
/// shape. The header names the element and its owner when the diagnostic is
/// attributed; un-attributed diagnostics carry `(inexact)`.
pub fn lines(catalog: &Catalog, diagnostics: &[Diagnostic], touched: &[ElId]) -> Vec<String> {
    let first = first_actionable(diagnostics, touched);
    let mut lines = Vec::new();
    for (index, diag) in diagnostics.iter().enumerate() {
        let mut header = format!(
            "  {}[{}] {}:{}",
            diag.level.as_str(),
            diag.code,
            diag.path,
            diag.line
        );
        if let Some(el) = diag.el {
            header.push_str(&format!("  {el}"));
            if let Some(owner) = diag.owner {
                let alias = catalog
                    .find(owner)
                    .and_then(|element| element.alias.as_ref())
                    .map(|alias| alias.to_string())
                    .unwrap_or_default();
                header.push_str(&format!("  (in {owner} {alias})"));
            }
        }
        if !diag.exact {
            header.push_str("  (inexact)");
        }
        if first == Some(index) {
            header.push_str("  (first actionable)");
        }
        lines.push(header);
        lines.push(format!("    {}", diag.message));
    }
    lines
}

/// The one diagnostic the next attempt should start from: an element the change
/// touched beats an untouched one, an exact attribution beats an inexact one, and
/// the first in the compiler's order is the final tie-break. Deterministic, so two
/// identical attempts mark the same line.
pub fn first_actionable(diagnostics: &[Diagnostic], touched: &[ElId]) -> Option<usize> {
    if diagnostics.is_empty() {
        return None;
    }
    diagnostics
        .iter()
        .position(|diag| {
            diag.el.is_some_and(|el| touched.contains(&el))
                || diag.owner.is_some_and(|owner| touched.contains(&owner))
        })
        .or_else(|| diagnostics.iter().position(|diag| diag.exact))
        .or(Some(0))
}

fn to_gate_diagnostics(diagnostics: &[Diagnostic], touched: &[ElId]) -> Vec<GateDiagnostic> {
    let first = first_actionable(diagnostics, touched);
    diagnostics
        .iter()
        .enumerate()
        .map(|(index, diag)| GateDiagnostic {
            code: diag.code.clone(),
            level: diag.level.as_str().to_string(),
            el: diag.el.map(|el| el.to_string()),
            owner: diag.owner.map(|owner| owner.to_string()),
            path: diag.path.clone(),
            line: diag.line,
            col: diag.col,
            message: diag.message.clone(),
            exact: diag.exact,
            first_actionable: first == Some(index),
        })
        .collect()
}

/// A report with no baseline: every diagnostic is `existing`, because `el check`
/// reports the current tree rather than a change. Used by `check`, not the gate.
pub fn gate_from_diagnostics(
    tier: Tier,
    diagnostics: &[Diagnostic],
    verdict: &str,
    ms: u64,
    touched: &[ElId],
) -> Gate {
    Gate {
        tier: tier.as_str().to_string(),
        new: 0,
        fixed: 0,
        existing: diagnostics.len(),
        verdict: verdict.to_string(),
        ms,
        diagnostics: to_gate_diagnostics(diagnostics, touched),
    }
}

pub(super) fn gate_report(
    tier: Tier,
    delta: &Delta,
    verdict: &str,
    ms: u64,
    touched: &[ElId],
) -> Gate {
    Gate {
        tier: tier.as_str().to_string(),
        new: delta.new.len(),
        fixed: delta.fixed.len(),
        existing: delta.existing.len(),
        verdict: verdict.to_string(),
        ms,
        diagnostics: to_gate_diagnostics(&delta.new, touched),
    }
}

#[cfg(test)]
#[path = "first_actionable_tests.rs"]
mod first_actionable_tests;
