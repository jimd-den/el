// The three L0 rules that can only be answered by the text as it will be written:
// a file over `ceiling` (FR-26), an element's body over `max_fn_lines`, and a
// module without a preamble when the policy requires one. They are measured on the
// projection, never on the disk, which is what makes them free and what lets a
// refusal happen before anything is written.
//
// The remedy is the point of this module. A violation that says "201 lines" is a
// wall; one that names the two largest extractable statements is a seam the user
// can act on (FR-73). When there is no such statement the honest answer is that
// the file has no seam, and it says so rather than inventing a suggestion.
//
// It deliberately does not decide what a *good* seam is. It ranks by size and by
// id, both content-determined, so the same catalog always gets the same advice.
// The preamble rule needs no measurement at all, but it does need the file list,
// which is why it belongs here and not with the catalog rules.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::BodyMode;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::plan::policy::Policy;
use crate::core::plan::violation::{Code, Violation};
use crate::core::render::Projection;

/// Push every size violation the projection has, in file then element order.
pub fn checks(
    catalog: &Catalog,
    projection: &Projection,
    policy: &Policy,
    out: &mut Vec<Violation>,
) {
    ceilings(catalog, projection, policy, out);
    fn_budgets(catalog, projection, policy, out);
    preambles(catalog, projection, policy, out);
}

fn ceilings(
    catalog: &Catalog,
    projection: &Projection,
    policy: &Policy,
    out: &mut Vec<Violation>,
) {
    for file in &projection.files {
        if !file.path.ends_with(".rs") {
            continue;
        }
        let lines = file.text.lines().count();
        if lines <= policy.ceiling {
            continue;
        }
        out.push(
            Violation::new(Code::Ceiling, extractable(catalog, projection, &file.path, policy))
                .sized(lines),
        );
    }
}

/// The remedy for an over-long file: the two largest extractable statements in
/// it, named by id, or the honest statement that there is no seam.
fn extractable(catalog: &Catalog, projection: &Projection, path: &str, policy: &Policy) -> String {
    let mut candidates: Vec<(u32, ElId)> = projection
        .spans
        .iter()
        .filter(|span| span.path == path && catalog.find_stmt(span.element).is_some())
        .map(|span| (span.last - span.first + 1, span.element))
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let named: Vec<String> = candidates
        .iter()
        .take(2)
        .map(|(_, id)| id.to_string())
        .collect();
    if named.is_empty() {
        return format!(
            "{path}: split it; no single statement in it is extractable (ceiling {})",
            policy.ceiling
        );
    }
    format!(
        "{path}: extract statement {}, or raise `ceiling` in el.policy (now {})",
        named.join(" or "),
        policy.ceiling
    )
}

fn fn_budgets(
    catalog: &Catalog,
    projection: &Projection,
    policy: &Policy,
    out: &mut Vec<Violation>,
) {
    for element in &catalog.elements {
        if !element.kind.takes_block() || element.mode == BodyMode::Opaque {
            continue;
        }
        let Some(span) = projection
            .spans
            .iter()
            .find(|span| span.element == element.id)
        else {
            continue;
        };
        // The declaration line and the closing brace are the item's, not the
        // body's, so the body is what is left between them.
        let body = (span.last - span.first + 1) as usize - 2;
        if body > policy.max_fn_lines {
            let remedy = match extractable_statements(catalog, projection, element.id) {
                Some(ids) => format!(
                    "extract statement {ids}; max_fn_lines is {}",
                    policy.max_fn_lines
                ),
                None => format!("extract a statement; max_fn_lines is {}", policy.max_fn_lines),
            };
            out.push(Violation::at(Code::FnBudget, element.id, remedy).sized(body));
        }
    }
}

/// The two largest statements directly in `owner`'s body, for a budget remedy.
fn extractable_statements(catalog: &Catalog, projection: &Projection, owner: ElId) -> Option<String> {
    let mut candidates: Vec<(u32, ElId)> = catalog
        .statements
        .iter()
        .filter(|stmt| stmt.owner == owner)
        .filter_map(|stmt| {
            projection
                .spans
                .iter()
                .find(|span| span.element == stmt.id)
                .map(|span| (span.last - span.first + 1, stmt.id))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    if candidates.is_empty() {
        return None;
    }
    Some(
        candidates
            .iter()
            .take(2)
            .map(|(_, id)| id.to_string())
            .collect::<Vec<_>>()
            .join(" or "),
    )
}

fn preambles(
    catalog: &Catalog,
    projection: &Projection,
    policy: &Policy,
    out: &mut Vec<Violation>,
) {
    if !policy.require_preamble {
        return;
    }
    for file in &projection.files {
        if !file.path.ends_with(".rs") {
            continue;
        }
        let module = catalog
            .elements
            .iter()
            .find(|element| element.kind == Kind::Module && element.module == file.path);
        if module.is_some_and(|module| !catalog.children_of(module.id).is_empty()) {
            continue;
        }
        let violation = match module {
            Some(module) => Violation::at(
                Code::Preamble,
                module.id,
                format!("add a statement to it that says why {} exists", file.path),
            ),
            None => Violation::new(
                Code::Preamble,
                format!("{} has no module element to carry a preamble", file.path),
            ),
        };
        out.push(violation);
    }
}

#[cfg(test)]
#[path = "rendered_tests.rs"]
mod rendered_tests;
