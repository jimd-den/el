// Presentation for a transaction that ends without committing: the diagnostic
// lines GATE §4.3 publishes, and the counts the envelope carries. Kept apart from
// `outcome` so the three endings there read as what they do — restore, journal,
// report — and the wording lives in one place instead of three.

use crate::app::report::{Gate, GateDiagnostic};
use crate::core::gate::{Delta, Diagnostic, Tier};
use crate::core::model::catalog::Catalog;

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// One header line and one message line per diagnostic, in the GATE §4.3
/// shape. The header names the element and its owner when the diagnostic is
/// attributed; un-attributed diagnostics carry `(inexact)`.
pub fn lines(catalog: &Catalog, diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut lines = Vec::new();
    for diag in diagnostics {
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
        lines.push(header);
        lines.push(format!("    {}", diag.message));
    }
    lines
}

fn to_gate_diagnostics(diagnostics: &[Diagnostic]) -> Vec<GateDiagnostic> {
    diagnostics
        .iter()
        .map(|diag| GateDiagnostic {
            code: diag.code.clone(),
            level: diag.level.as_str().to_string(),
            el: diag.el.map(|el| el.to_string()),
            owner: diag.owner.map(|owner| owner.to_string()),
            path: diag.path.clone(),
            line: diag.line,
            col: diag.col,
            message: diag.message.clone(),
            exact: diag.exact,
        })
        .collect()
}

pub(super) fn gate_report(tier: Tier, delta: &Delta, verdict: &str, ms: u64) -> Gate {
    Gate {
        tier: tier.as_str().to_string(),
        new: delta.new.len(),
        fixed: delta.fixed.len(),
        existing: delta.existing.len(),
        verdict: verdict.to_string(),
        ms,
        diagnostics: to_gate_diagnostics(&delta.new),
    }
}

