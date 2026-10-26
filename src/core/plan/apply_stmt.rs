// The statement half of `apply`: add, edit, move, and the one rule that is
// enforced here as well as in `request` — a statement cannot cross into another
// element's body. Refusing it in both places is deliberate: `plan` asks first
// because the message is better while the user still recognises the ids, and this
// function refuses again so that it is total on its own, not only when called
// behind the gate.
//
// A move *within* a body is a re-parent and a position: `to` is the element (a
// top-level position) or a statement (nested inside it). Nesting is the model's
// parent link, not punctuation, so nothing here touches the text.
//
// It deliberately does not remove the statement from its old position by string
// surgery, and it does not renumber siblings into gaps: `children_of` sorts by
// (index, id), so an append with a stale index still reads in a defined order.

use super::violation::{Code, Violation};
use crate::core::model::catalog::{self, Catalog};
use crate::core::model::change::Change;
use crate::core::model::id::ElId;

/// Carry out `change` if it touches a statement; other variants are a no-op.
pub fn apply(catalog: &mut Catalog, change: &Change) -> Result<(), Violation> {
    match change {
        Change::AddStmt { owner, text } => catalog
            .add_stmt(*owner, text)
            .map(|_| ())
            .map_err(catalog_error),
        Change::EditStmt { id, text } => {
            let stmt = catalog
                .statements
                .iter_mut()
                .find(|stmt| stmt.id == *id)
                .ok_or_else(|| unknown(*id))?;
            stmt.text = text.clone();
            Ok(())
        }
        Change::MoveStmt { id, to } => move_stmt(catalog, *id, *to),
        _ => Ok(()),
    }
}

fn move_stmt(catalog: &mut Catalog, id: ElId, to: ElId) -> Result<(), Violation> {
    let owner = match catalog.find_stmt(id) {
        Some(stmt) => stmt.owner,
        None => return Err(unknown(id)),
    };

    let (target_owner, parent) = match catalog.find(to) {
        Some(element) => (element.id, None),
        None => match catalog.find_stmt(to) {
            Some(parent) => (parent.owner, Some(to)),
            None => return Err(unknown(to)),
        },
    };
    if target_owner != owner {
        return Err(Violation::at(
            Code::CrossBodyMove,
            id,
            format!(
                "it belongs to {owner}; add the text to {target_owner} and remove it here instead"
            ),
        ));
    }

    let index = catalog.children_of(to).len() as u32;
    if let Some(stmt) = catalog.statements.iter_mut().find(|stmt| stmt.id == id) {
        stmt.parent = parent;
        stmt.index = index;
    }
    Ok(())
}

fn unknown(id: ElId) -> Violation {
    Violation::new(Code::UnknownRef, format!("{id} does not exist"))
}

fn catalog_error(error: catalog::Error) -> Violation {
    match error {
        catalog::Error::UnknownId(id) => unknown(id),
        catalog::Error::DuplicateAlias { existing, alias } => Violation::at(
            Code::DuplicateAlias,
            existing,
            format!("{alias} is already taken by that element"),
        ),
        catalog::Error::BadSignature { signature } => Violation::new(
            Code::BadIdent,
            format!("{signature:?} does not start with its kind's keyword, or names nothing"),
        ),
    }
}

#[cfg(test)]
#[path = "apply_stmt_tests.rs"]
mod apply_stmt_tests;
