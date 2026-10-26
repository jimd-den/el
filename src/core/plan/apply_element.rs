// The element half of `apply`: add, edit, move, rename, remove, and the mapping
// from the catalog's own refusal (`model::catalog::Error`) to the violation a user
// sees. It is *total over `Catalog`* in the sense that matters: every input either
// carries the change out or returns a violation, and no input panics.
//
// The mutations themselves live in the catalog (`edit`, `identity`), because that
// is where the invariants they carry — id stability, alias uniqueness — already
// are. This module is the adapter between the request vocabulary and those calls;
// it decides nothing about legality, which is `validate`'s job one step later.
//
// It deliberately accepts a head for an edit without re-parsing it as a
// signature: the name is what a rename changes, and an edit that could change the
// name would be a rename wearing a different word. A head that opens a delimiter
// it never closes is refused later, by `model.unbalanced`.

use super::violation::{Code, Violation};
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::catalog::{self, Catalog};
use crate::core::model::change::Change;
use crate::core::model::element::NewElement;
use crate::core::model::id::ElId;

/// Carry out `change` if it touches an element; other variants are left for
/// `apply_stmt` and are a no-op here.
pub fn apply(catalog: &mut Catalog, change: &Change) -> Result<(), Violation> {
    match change {
        Change::AddElement {
            kind,
            module,
            signature,
            visibility,
            body,
            mode,
        } => {
            let id = catalog
                .add_element(&NewElement {
                    kind: *kind,
                    module,
                    signature,
                    visibility: *visibility,
                })
                .map_err(catalog_error)?;
            if let Some(element) = catalog.find_mut(id) {
                element.mode = *mode;
            }
            add_body(catalog, id, body)
        }
        Change::EditBody { id, body, mode } if catalog.find(*id).is_some() => {
            remove_children(catalog, *id)?;
            if let Some(element) = catalog.find_mut(*id)
                && let Some(mode) = mode
            {
                element.mode = *mode;
            }
            add_body(catalog, *id, body)
        }
        Change::EditElement {
            id,
            head,
            visibility,
            body,
            mode,
        } => {
            {
                let element = catalog.find_mut(*id).ok_or_else(|| unknown(*id))?;
                if let Some(head) = head {
                    element.head = head.clone();
                }
                if let Some(visibility) = visibility {
                    element.visibility = *visibility;
                }
                if let Some(mode) = mode {
                    element.mode = *mode;
                }
            }
            if let Some(body) = body {
                remove_children(catalog, *id)?;
                add_body(catalog, *id, body)?;
            }
            Ok(())
        }
        Change::MoveElement { id, module } => catalog.move_to(*id, module).map_err(catalog_error),
        Change::Rename { id, name } => catalog.rename(*id, name).map_err(catalog_error),
        Change::Remove { id } => catalog.remove(*id).map_err(catalog_error),
        _ => Ok(()),
    }
}

/// Insert a parsed body under `owner`, preserving the nesting. The catalog's
/// `add_child_stmt` derives the owner from the parent, so a nested statement can
/// never be filed under the wrong element.
fn add_body(catalog: &mut Catalog, owner: ElId, body: &Body) -> Result<(), Violation> {
    for stmt in &body.statements {
        let id = catalog
            .add_stmt(owner, &stmt.text)
            .map_err(catalog_error)?;
        add_children(catalog, id, &stmt.children)?;
    }
    Ok(())
}

fn add_children(catalog: &mut Catalog, parent: ElId, children: &[BodyStmt]) -> Result<(), Violation> {
    for child in children {
        let id = catalog
            .add_child_stmt(parent, &child.text)
            .map_err(catalog_error)?;
        add_children(catalog, id, &child.children)?;
    }
    Ok(())
}

/// Replace a body by removing what was there, deepest first through the
/// catalog's own recursive `remove`, then inserting the new tree.
fn remove_children(catalog: &mut Catalog, owner: ElId) -> Result<(), Violation> {
    let children: Vec<ElId> = catalog.children_of(owner).iter().map(|stmt| stmt.id).collect();
    for child in children {
        catalog.remove(child).map_err(catalog_error)?;
    }
    Ok(())
}

fn unknown(id: ElId) -> Violation {
    Violation::new(Code::UnknownRef, format!("{id} does not exist"))
}

/// The catalog's refusal, said in L0's words.
pub(super) fn catalog_error(error: catalog::Error) -> Violation {
    match error {
        catalog::Error::DuplicateAlias { existing, alias } => Violation::at(
            Code::DuplicateAlias,
            existing,
            format!("{alias} is already taken by that element"),
        ),
        catalog::Error::BadSignature { signature } => Violation::new(
            Code::BadIdent,
            format!("{signature:?} does not start with its kind's keyword, or names nothing"),
        ),
        catalog::Error::UnknownId(id) => unknown(id),
    }
}

#[cfg(test)]
#[path = "apply_element_tests.rs"]
mod apply_element_tests;
