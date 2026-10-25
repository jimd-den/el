// The catalog is the source of truth (ADR-0002): every element and every
// statement `el` knows about, plus the counter that issues permanent ids.
//
// This file holds the state and the read-only questions you can ask it. The
// mutations live in `edit` because they carry the invariants (id stability, alias
// uniqueness) that make the model worth having; the on-disk text form lives in
// `codec`. Keeping them apart is what lets each be small enough to read.
//
// Invariant: every `ElId` in the catalog is unique across elements and
// statements, and `next_id` is strictly greater than every issued id.

use super::element::Element;
use super::id::ElId;
use super::stmt::Stmt;

pub mod codec;
mod edit;
mod identity;

/// Why a mutation was refused. The colliding id is named, because a user with a
/// duplicate alias needs to know which element already holds it (FR-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    DuplicateAlias { existing: ElId, alias: String },
    UnknownId(ElId),
    EmptyName,
}

#[derive(Debug)]
pub struct Catalog {
    pub elements: Vec<Element>,
    pub statements: Vec<Stmt>,
    next_id: u32,
}

impl PartialEq for Catalog {
    /// Value equality: two catalogs with the same content are the same catalog,
    /// whatever order each happened to build its vectors in. Decoding rebuilds
    /// records in canonical order, so comparing vector order would make a
    /// successful round trip look like a failure.
    fn eq(&self, other: &Self) -> bool {
        self.next_id == other.next_id
            && by_id(&self.elements, |element| element.id)
                == by_id(&other.elements, |element| element.id)
            && by_id(&self.statements, |stmt| stmt.id)
                == by_id(&other.statements, |stmt| stmt.id)
    }
}

impl Eq for Catalog {}

/// The items as references, ordered by id — the canonical view for comparison.
fn by_id<T>(items: &[T], id: fn(&T) -> ElId) -> Vec<&T> {
    let mut ordered: Vec<&T> = items.iter().collect();
    ordered.sort_by_key(|item| id(item));
    ordered
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            statements: Vec::new(),
            next_id: 1,
        }
    }

    /// Issue the next id. Monotonic for the life of the catalog: an id that has
    /// been issued is never handed out twice, even after a removal (FR-1).
    pub(crate) fn alloc_id(&mut self) -> ElId {
        let id = ElId(self.next_id);
        self.next_id += 1;
        id
    }

    pub(crate) fn next_id(&self) -> u32 {
        self.next_id
    }

    pub(crate) fn restore_counter(&mut self, next: u32) {
        self.next_id = next;
    }

    pub fn find(&self, id: ElId) -> Option<&Element> {
        self.elements.iter().find(|element| element.id == id)
    }

    pub fn find_mut(&mut self, id: ElId) -> Option<&mut Element> {
        self.elements.iter_mut().find(|element| element.id == id)
    }

    pub fn find_stmt(&self, id: ElId) -> Option<&Stmt> {
        self.statements.iter().find(|stmt| stmt.id == id)
    }

    /// Direct children of a statement or element, in source order.
    pub fn children_of(&self, parent: ElId) -> Vec<&Stmt> {
        let mut children: Vec<&Stmt> = self
            .statements
            .iter()
            .filter(|stmt| stmt.parent == Some(parent) || (stmt.parent.is_none() && stmt.owner == parent))
            .collect();
        children.sort_by_key(|stmt| (stmt.index, stmt.id));
        children
    }

    /// Depth-first walk of a statement and everything under it (FR-62).
    pub fn walk(&self, root: ElId) -> Vec<&Stmt> {
        let mut seen = Vec::new();
        let mut pending = vec![root];

        while let Some(id) = pending.pop() {
            if let Some(stmt) = self.find_stmt(id) {
                seen.push(stmt);
                let children = self.children_of(id);
                for child in children.into_iter().rev() {
                    pending.push(child.id);
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::element::Visibility;
    use crate::core::model::kind::Kind;

    fn catalog_with_tree() -> (Catalog, ElId, ElId, ElId) {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(Kind::Function, "src/main.rs", "main", Visibility::Private)
            .expect("add");
        let outer = catalog.add_stmt(owner, "if c {").expect("add");
        let inner = catalog.add_child_stmt(outer, "a();").expect("add");

        (catalog, owner, outer, inner)
    }

    #[test]
    fn children_come_back_in_source_order() {
        let (mut catalog, owner, _, _) = catalog_with_tree();
        let second = catalog.add_stmt(owner, "b();").expect("add");

        let children = catalog.children_of(owner);

        assert_eq!(children.len(), 2);
        assert_eq!(children[1].id, second);
    }

    #[test]
    fn a_walk_visits_a_statement_before_its_children() {
        let (catalog, _, outer, inner) = catalog_with_tree();

        let walked: Vec<ElId> = catalog.walk(outer).iter().map(|stmt| stmt.id).collect();

        assert_eq!(walked, vec![outer, inner]);
    }

    #[test]
    fn an_empty_catalog_has_no_elements() {
        assert!(Catalog::new().elements.is_empty());
    }
}
