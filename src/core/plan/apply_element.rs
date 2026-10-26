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
use crate::core::model::catalog::{self, Catalog};
use crate::core::model::change::Change;
use crate::core::model::element::NewElement;

/// Carry out `change` if it touches an element; other variants are left for
/// `apply_stmt` and are a no-op here.
pub fn apply(catalog: &mut Catalog, change: &Change) -> Result<(), Violation> {
    match change {
        Change::AddElement {
            kind,
            module,
            signature,
            visibility,
        } => catalog
            .add_element(&NewElement {
                kind: *kind,
                module,
                signature,
                visibility: *visibility,
            })
            .map(|_| ())
            .map_err(catalog_error),
        Change::EditElement { id, head } => {
            let element = catalog.find_mut(*id).ok_or_else(|| unknown(*id))?;
            element.head = head.clone();
            Ok(())
        }
        Change::MoveElement { id, module } => catalog.move_to(*id, module).map_err(catalog_error),
        Change::Rename { id, name } => catalog.rename(*id, name).map_err(catalog_error),
        Change::Remove { id } => catalog.remove(*id).map_err(catalog_error),
        _ => Ok(()),
    }
}

fn unknown(id: crate::core::model::id::ElId) -> Violation {
    Violation::new(Code::UnknownRef, format!("{id} does not exist"))
}

/// The catalog's refusal, said in L0's words.
fn catalog_error(error: catalog::Error) -> Violation {
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
