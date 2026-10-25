// Creating elements and statements. This is where the counter is spent, and the
// two rules that make creation safe live here:
//
//   1. An id is issued from a monotonic counter, so a deleted id is never handed
//      out again (FR-1).
//   2. An alias is unique across the catalog, including across kinds (FR-2); a
//      collision is refused and names the element that already holds it.
//
// A refused `add_element` still consumed an id. That is deliberate: ids are
// cheap, and rewinding the counter to reclaim one would be the first step toward
// reissuing it.
//
// It does not validate Rust: a signature that does not start with its kind's
// keyword, or a named kind with no identifier, is refused because the element
// could not be addressed or rendered — but whether `fn apply` is *legal* is L0's
// question, one layer out.

use super::identity::holder_of;
use super::{Catalog, Error};
use crate::core::model::element::{Element, NewElement};
use crate::core::model::id::ElId;
use crate::core::model::stmt::Stmt;

impl Catalog {
    /// Add an item; the catalog issues its id and derives its alias.
    pub fn add_element(&mut self, new: &NewElement<'_>) -> Result<ElId, Error> {
        let id = self.alloc_id();
        let element = Element::create(id, new).ok_or_else(|| Error::BadSignature {
            signature: new.signature.to_string(),
        })?;
        if let Some(alias) = &element.alias
            && let Some(existing) = holder_of(self, alias.as_str(), id)
        {
            return Err(Error::DuplicateAlias {
                existing,
                alias: alias.to_string(),
            });
        }
        self.elements.push(element);
        Ok(id)
    }

    /// Append a statement to an element's body, after its existing statements.
    pub fn add_stmt(&mut self, owner: ElId, text: &str) -> Result<ElId, Error> {
        if self.find(owner).is_none() {
            return Err(Error::UnknownId(owner));
        }
        self.push_stmt(owner, None, text)
    }

    /// Append a statement inside another statement's block (FR-62).
    pub fn add_child_stmt(&mut self, parent: ElId, text: &str) -> Result<ElId, Error> {
        let owner = match self.find_stmt(parent) {
            Some(stmt) => stmt.owner,
            None => return Err(Error::UnknownId(parent)),
        };
        self.push_stmt(owner, Some(parent), text)
    }

    fn push_stmt(&mut self, owner: ElId, parent: Option<ElId>, text: &str) -> Result<ElId, Error> {
        let index = self.children_of(parent.unwrap_or(owner)).len() as u32;
        let id = self.alloc_id();
        self.statements.push(Stmt::new(id, owner, parent, index, text));
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::element::{NewElement, Visibility};
    use crate::core::model::kind::Kind;

    fn new_el<'a>(kind: Kind, module: &'a str, signature: &'a str, visibility: Visibility) -> NewElement<'a> {
        NewElement { kind, module, signature, visibility }
    }

    #[test]
    fn id_assigned_and_never_reused() {
        let mut catalog = Catalog::new();
        let doomed = catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn gone", Visibility::Private))
            .expect("add");
        catalog.remove(doomed).expect("remove");

        let fresh = catalog
            .add_element(&new_el(Kind::Struct, "src/a.rs", "struct S", Visibility::Private))
            .expect("add");

        assert_eq!(doomed, ElId(1));
        assert_eq!(fresh, ElId(2), "el-0001 must not be handed out again");
    }

    #[test]
    fn removing_a_statement_does_not_reissue_its_id() {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn run", Visibility::Private))
            .expect("add");
        let stmt = catalog.add_stmt(owner, "a();").expect("add");
        catalog.remove(stmt).expect("remove");

        let next = catalog.add_stmt(owner, "b();").expect("add");

        assert_eq!(next, ElId(3));
    }

    #[test]
    fn alias_is_unique_across_kinds() {
        let mut catalog = Catalog::new();
        let function = catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn thing", Visibility::Private))
            .expect("add");
        let structure = catalog
            .add_element(&new_el(Kind::Struct, "src/a.rs", "struct thing", Visibility::Private))
            .expect("different kind, different alias");

        let refused =
            catalog.add_element(&new_el(Kind::Function, "src/a.rs", "fn thing", Visibility::Private));

        assert_ne!(function, structure);
        assert_eq!(
            refused,
            Err(Error::DuplicateAlias {
                existing: function,
                alias: "a::fn/thing".to_string(),
            })
        );
    }

    #[test]
    fn an_unnamed_kind_needs_no_name() {
        let mut catalog = Catalog::new();

        let id = catalog
            .add_element(&new_el(Kind::Impl, "src/a.rs", "impl", Visibility::Private))
            .expect("impl has no name");

        assert_eq!(catalog.find(id).expect("added").alias, None);
    }

    #[test]
    fn a_signature_with_no_name_is_refused() {
        let mut catalog = Catalog::new();

        assert_eq!(
            catalog.add_element(&new_el(Kind::Function, "src/a.rs", "fn", Visibility::Private)),
            Err(Error::BadSignature {
                signature: "fn".to_string()
            })
        );
    }

    #[test]
    fn a_signature_for_the_wrong_kind_is_refused() {
        let mut catalog = Catalog::new();

        let refused = catalog.add_element(&new_el(
            Kind::Function,
            "src/a.rs",
            "struct Point",
            Visibility::Private,
        ));

        assert_eq!(
            refused,
            Err(Error::BadSignature {
                signature: "struct Point".to_string()
            })
        );
    }

    #[test]
    fn statement_tree_nests() {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(&new_el(Kind::Function, "src/a.rs", "fn run", Visibility::Private))
            .expect("add");
        let outer = catalog.add_stmt(owner, "loop {").expect("add");
        let middle = catalog.add_child_stmt(outer, "if c {").expect("add");
        let inner = catalog.add_child_stmt(middle, "break;").expect("add");

        assert_eq!(middle.0, outer.0 + 1);
        assert!(inner != middle && inner != outer);
        assert_eq!(catalog.walk(outer).len(), 3);
        assert_eq!(catalog.walk(outer).last().map(|stmt| stmt.id), Some(inner));
    }
}
