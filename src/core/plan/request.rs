// L0, second half: the rules that are about the *request*, not the result. A
// change can be refused before it is applied when the thing being asked for is
// impossible in principle — moving a statement into another function, adding a
// dependency the policy denies, or deleting a name that other code still uses.
//
// Asking first is what makes the refusal cheap and the message precise: the
// catalog is still the one the user knows, so every id in the message is one they
// can look up. It is also why these three live apart from `validate::state` —
// that one answers "is this catalog legal", this one answers "is this request
// answerable".
//
// It deliberately does not apply anything, and it does not guess: an unscannable
// body yields no dangling-reference claim here, because `validate::state` will
// refuse that body with `model.unbalanced` a moment later.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::BodyMode;
use crate::core::model::change::Change;
use crate::core::model::id::ElId;
use crate::core::plan::policy::Policy;
use crate::core::plan::refs::{self, Site};
use crate::core::plan::violation::{Code, Violation};

/// The three rules that depend on what was asked for, in GATE.md's order.
pub fn request(change: &Change, catalog: &Catalog, policy: &Policy) -> Vec<Violation> {
    let mut found = Vec::new();

    opaque_change(change, catalog, &mut found);
    match change {
        Change::MoveStmt { id, to } => cross_body_move(*id, *to, catalog, &mut found),
        Change::Remove { id } => dangling_ref(*id, catalog, &mut found),
        Change::AddDep { name } if !policy.allows_dep(name) => found.push(Violation::new(
            Code::DepDenied,
            format!("`{name}` is not allowed by el.policy; list it under `deps` to allow it"),
        )),
        _ => {}
    }

    found
}

/// An element added to an adopted file would exist in the catalog and not in the
/// file: the opaque module renders its stored bytes and nothing else. Refuse it
/// and name the file-level edit that does work.
fn opaque_change(change: &Change, catalog: &Catalog, out: &mut Vec<Violation>) {
    let target = match change {
        Change::AddElement { module, .. } | Change::MoveElement { module, .. } => {
            opaque_file(module, catalog).or_else(|| opaque_ancestor(module, catalog))
        }
        Change::AddStmt { owner, .. } => catalog.find(*owner).filter(|element| {
            element.mode == BodyMode::Opaque
        }),
        _ => None,
    };
    if let Some(module) = target {
        out.push(Violation::at(
            Code::Opaque,
            module.id,
            format!(
                "{} is opaque; edit the whole file with `el edit {} --body <file> --body-mode opaque`",
                module.module, module.id
            ),
        ));
    }
}

/// An opaque file that would have to declare the new descendant but cannot be
/// updated by an element change: `src/a/b.rs` needs `mod b;` in `src/a.rs` or
/// `src/a/mod.rs`, and either may be opaque.
fn opaque_ancestor<'a>(
    path: &str,
    catalog: &'a Catalog,
) -> Option<&'a crate::core::model::element::Element> {
    let stem = path.strip_suffix(".rs")?;
    let parts: Vec<&str> = stem.split('/').collect();
    for end in (1..parts.len()).rev() {
        let prefix = parts[..end].join("/");
        for candidate in [format!("{prefix}.rs"), format!("{prefix}/mod.rs")] {
            if let Some(module) = opaque_file(&candidate, catalog) {
                return Some(module);
            }
        }
    }
    None
}

fn opaque_file<'a>(
    path: &str,
    catalog: &'a Catalog,
) -> Option<&'a crate::core::model::element::Element> {
    catalog.elements.iter().find(|element| {
        element.kind == crate::core::model::kind::Kind::Module
            && element.module == path
            && element.mode == BodyMode::Opaque
    })
}

fn cross_body_move(id: ElId, to: ElId, catalog: &Catalog, out: &mut Vec<Violation>) {
    let Some(stmt) = catalog.find_stmt(id) else { return };
    let target = match (catalog.find(to), catalog.find_stmt(to)) {
        (Some(_), _) => to,
        (None, Some(parent)) => parent.owner,
        (None, None) => {
            out.push(Violation::at(
                Code::UnknownRef,
                id,
                format!("{to} does not exist, so there is nowhere to move to"),
            ));
            return;
        }
    };
    if target != stmt.owner {
        out.push(Violation::at(
            Code::CrossBodyMove,
            id,
            format!(
                "it belongs to {}; add the text to {target} and remove it here instead",
                stmt.owner
            ),
        ));
    }
}

fn dangling_ref(id: ElId, catalog: &Catalog, out: &mut Vec<Violation>) {
    let Some(element) = catalog.find(id) else { return };
    let Some(name) = element.name.clone() else { return };
    let Some(hits) = refs::references(catalog, &name) else { return };

    let survivor = hits
        .iter()
        .find(|reference| !is_inside(reference.site, id, catalog));
    if let Some(survivor) = survivor {
        out.push(Violation::at(
            Code::DanglingRef,
            survivor.site.element(),
            format!("still refers to {name} ({id}), which this change removes; rewrite it first"),
        ));
    }
}

/// Whether a site is part of what removal takes with it: the element's own head,
/// or a statement in its own body.
fn is_inside(site: Site, removed: ElId, catalog: &Catalog) -> bool {
    match site {
        Site::Head(element) => element == removed,
        Site::Stmt(id) => catalog
            .find_stmt(id)
            .is_some_and(|stmt| stmt.owner == removed),
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
