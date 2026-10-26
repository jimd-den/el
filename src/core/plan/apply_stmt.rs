// The statement half of `apply`: insert at an anchor, edit one statement, reorder,
// re-parent, and the one rule enforced here as well as in `request` — a statement
// cannot cross into another element's body. Refusing it in both places is
// deliberate: `plan` asks first because the message is better while the user still
// recognises the ids, and this function refuses again so that it is total on its
// own, not only when called behind the gate.
//
// Nesting is the model's parent link, not punctuation. An insert or reorder
// renumbers its sibling list so source order is always 0..n; ids never move.
//
// It deliberately does not split text or decide whether a statement is legal
// Rust. `text::grammar` made the body tree; the compiler is the arbiter.

use super::violation::{Code, Violation};
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::catalog::{self, Catalog};
use crate::core::model::change::{Anchor, Change};
use crate::core::model::id::ElId;

/// Carry out `change` if it touches a statement; other variants are a no-op.
pub fn apply(catalog: &mut Catalog, change: &Change) -> Result<(), Violation> {
    match change {
        Change::AddStmt { owner, body, anchor } => {
            let stmt = one(body)?;
            let (parent, position) = resolve(catalog, *owner, anchor)?;
            let id = catalog
                .insert_stmt(*owner, parent, position, &stmt.text)
                .map_err(catalog_error)?;
            add_children(catalog, id, &stmt.children)
        }
        Change::EditStmt { id, body } => edit(catalog, *id, body),
        Change::EditBody { id, body, .. } if catalog.find(*id).is_none() => edit(catalog, *id, body),
        Change::ReorderStmt { id, after } => reorder(catalog, *id, *after),
        Change::MoveStmt { id, to } => move_stmt(catalog, *id, *to),
        _ => Ok(()),
    }
}

fn edit(catalog: &mut Catalog, id: ElId, body: &Body) -> Result<(), Violation> {
    let stmt = one(body)?;
    if catalog.find_stmt(id).is_none() {
        return Err(unknown(id));
    }
    let children: Vec<ElId> = catalog
        .children_of(id)
        .iter()
        .map(|child| child.id)
        .collect();
    for child in children {
        catalog.remove(child).map_err(catalog_error)?;
    }
    if let Some(existing) = catalog.statements.iter_mut().find(|stmt| stmt.id == id) {
        existing.text = stmt.text.clone();
    }
    add_children(catalog, id, &stmt.children)
}

fn reorder(catalog: &mut Catalog, id: ElId, after: ElId) -> Result<(), Violation> {
    let Some(stmt) = catalog.find_stmt(id) else {
        return Err(unknown(id));
    };
    let Some(anchor) = catalog.find_stmt(after) else {
        return Err(unknown(after));
    };
    if stmt.owner != anchor.owner {
        return Err(Violation::at(
            Code::CrossBodyMove,
            id,
            format!("it belongs to {}; add the text to {} instead", stmt.owner, anchor.owner),
        ));
    }
    catalog.reorder_after(id, after).map_err(catalog_error)
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
            format!("it belongs to {owner}; add the text to {target_owner} and remove it here instead"),
        ));
    }
    let position = catalog.children_of(parent.unwrap_or(target_owner)).len();
    catalog
        .move_stmt(id, target_owner, parent, position)
        .map_err(catalog_error)
}

fn resolve(catalog: &Catalog, owner: ElId, anchor: &Anchor) -> Result<(Option<ElId>, usize), Violation> {
    if catalog.find(owner).is_none() {
        return Err(unknown(owner));
    }
    match anchor {
        Anchor::End => Ok((None, catalog.children_of(owner).len())),
        Anchor::At(position) => Ok((None, *position)),
        Anchor::After(id) | Anchor::Before(id) => {
            let (parent, siblings) = match catalog.find_stmt(*id) {
                Some(stmt) if stmt.owner == owner => (stmt.parent, catalog.children_of(stmt.parent.unwrap_or(owner))),
                Some(_) => {
                    return Err(Violation::at(
                        Code::CrossBodyMove,
                        *id,
                        "the anchor belongs to another body",
                    ));
                }
                None if *id == owner => (None, catalog.children_of(owner)),
                None => return Err(unknown(*id)),
            };
            let index = siblings
                .iter()
                .position(|stmt| stmt.id == *id)
                .ok_or_else(|| unknown(*id))?;
            let position = match anchor {
                Anchor::After(_) => index + 1,
                Anchor::Before(_) => index,
                _ => unreachable!(),
            };
            Ok((parent, position))
        }
    }
}

fn one(body: &Body) -> Result<&BodyStmt, Violation> {
    if body.statements.len() != 1 {
        return Err(Violation::new(
            Code::BadStmt,
            "this verb changes one statement; give exactly one top-level statement",
        ));
    }
    Ok(&body.statements[0])
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

fn unknown(id: ElId) -> Violation {
    Violation::new(Code::UnknownRef, format!("{id} does not exist"))
}

pub(super) fn catalog_error(error: catalog::Error) -> Violation {
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
#[path = "apply_stmt_anchor_tests.rs"]
mod apply_stmt_anchor_tests;

#[cfg(test)]
#[path = "apply_stmt_tests.rs"]
mod apply_stmt_tests;
