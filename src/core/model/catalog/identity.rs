// Changing what an element *is called* or *where it lives*, without changing what
// it *is*. These are the operations an agent performs most often, and they all
// have to preserve the one thing the model promises: the id (FR-1).
//
// The alias is derived, so any rename or move has to be checked against the whole
// catalog before it is applied — otherwise two elements could end up sharing a
// name (FR-2), and every later reference by alias would be ambiguous.
//
// Removal is here too, because it is the operation that makes "never reused"
// a claim rather than an accident: deleting an element does not lower the
// counter.

use super::{Catalog, Error};
use crate::core::model::element::Element;
use crate::core::model::id::ElId;

impl Catalog {
    /// Remove an element (and every statement in its body) or a statement (and
    /// everything nested inside it). Issued ids are never reissued.
    pub fn remove(&mut self, id: ElId) -> Result<(), Error> {
        if self.find(id).is_some() {
            self.elements.retain(|element| element.id != id);
            self.statements.retain(|stmt| stmt.owner != id);
            return Ok(());
        }
        if self.find_stmt(id).is_none() {
            return Err(Error::UnknownId(id));
        }
        let doomed: Vec<ElId> = self.walk(id).iter().map(|stmt| stmt.id).collect();
        self.statements.retain(|stmt| !doomed.contains(&stmt.id));
        Ok(())
    }

    pub fn rename(&mut self, id: ElId, name: &str) -> Result<(), Error> {
        let mut candidate = self.find(id).ok_or(Error::UnknownId(id))?.clone();
        candidate.rename(name);
        self.accept_alias_change(&candidate)?;
        if let Some(element) = self.find_mut(id) {
            element.rename(name);
        }
        Ok(())
    }

    pub fn move_to(&mut self, id: ElId, module: &str) -> Result<(), Error> {
        let mut candidate = self.find(id).ok_or(Error::UnknownId(id))?.clone();
        candidate.move_to(module);
        self.accept_alias_change(&candidate)?;
        if let Some(element) = self.find_mut(id) {
            element.move_to(module);
        }
        Ok(())
    }

    fn accept_alias_change(&self, candidate: &Element) -> Result<(), Error> {
        match &candidate.alias {
            Some(alias) => match holder_of(self, alias.as_str(), candidate.id) {
                Some(existing) => Err(Error::DuplicateAlias {
                    existing,
                    alias: alias.to_string(),
                }),
                None => Ok(()),
            },
            None => Ok(()),
        }
    }
}

/// The element already holding `alias`, if any, ignoring `ignoring`.
pub(super) fn holder_of(catalog: &Catalog, alias: &str, ignoring: ElId) -> Option<ElId> {
    catalog
        .elements
        .iter()
        .find(|element| {
            element.id != ignoring
                && element.alias.as_ref().is_some_and(|held| held.as_str() == alias)
        })
        .map(|element| element.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::element::NewElement;

    fn new_el<'a>(kind: Kind, module: &'a str, signature: &'a str, visibility: Visibility) -> NewElement<'a> {
        NewElement { kind, module, signature, visibility }
    }
    use crate::core::model::element::Visibility;
    use crate::core::model::kind::Kind;

    fn two_elements() -> Catalog {
        let mut catalog = Catalog::new();
        catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn run", Visibility::Private))
            .expect("add");
        catalog
            .add_element(&new_el(Kind::Const, "src/a.rs", "const LIMIT", Visibility::Private))
            .expect("add");
        catalog
    }

    #[test]
    fn id_survives_rename_and_move() {
        let mut catalog = two_elements();

        catalog.rename(ElId(1), "dispatch").expect("rename");
        catalog.move_to(ElId(1), "src/b.rs").expect("move");

        let element = catalog.find(ElId(1)).expect("still there");
        assert_eq!(element.id, ElId(1));
        assert_eq!(element.alias.as_ref().map(|a| a.as_str()), Some("b::fn/dispatch"));
    }

    #[test]
    fn a_rename_into_a_taken_alias_is_refused() {
        let mut catalog = Catalog::new();
        let first = catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn one", Visibility::Private))
            .expect("add");
        catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn two", Visibility::Private))
            .expect("add");

        let refused = catalog.rename(ElId(2), "one");

        assert_eq!(
            refused,
            Err(Error::DuplicateAlias {
                existing: first,
                alias: "a::fn/one".to_string(),
            })
        );
    }

    #[test]
    fn removing_an_element_removes_its_body() {
        let mut catalog = two_elements();
        catalog.add_stmt(ElId(1), "a();").expect("add");

        catalog.remove(ElId(1)).expect("remove");

        assert!(catalog.statements.is_empty());
        assert!(catalog.find(ElId(1)).is_none());
    }

    #[test]
    fn removing_a_statement_removes_its_children() {
        let mut catalog = two_elements();
        let outer = catalog.add_stmt(ElId(1), "loop {").expect("add");
        catalog.add_child_stmt(outer, "break;").expect("add");

        catalog.remove(outer).expect("remove");

        assert!(catalog.statements.is_empty());
    }

    #[test]
    fn removing_an_unknown_id_is_refused() {
        let mut catalog = two_elements();

        assert_eq!(catalog.remove(ElId(99)), Err(Error::UnknownId(ElId(99))));
    }
}
