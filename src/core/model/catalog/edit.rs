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
        let position = self.children_of(owner).len();
        self.insert_stmt(owner, None, position, text)
    }

    /// Append a statement inside another statement's block (FR-62).
    pub fn add_child_stmt(&mut self, parent: ElId, text: &str) -> Result<ElId, Error> {
        let owner = match self.find_stmt(parent) {
            Some(stmt) => stmt.owner,
            None => return Err(Error::UnknownId(parent)),
        };
        let position = self.children_of(parent).len();
        self.insert_stmt(owner, Some(parent), position, text)
    }

    /// Insert `text` at `position` among `parent`'s children (or the element's
    /// top-level statements when `parent` is `None`). Existing siblings keep their
    /// ids and are renumbered, so source order is always `0..n`.
    pub fn insert_stmt(
        &mut self,
        owner: ElId,
        parent: Option<ElId>,
        position: usize,
        text: &str,
    ) -> Result<ElId, Error> {
        if self.find(owner).is_none() {
            return Err(Error::UnknownId(owner));
        }
        if let Some(parent) = parent {
            let Some(stmt) = self.find_stmt(parent) else {
                return Err(Error::UnknownId(parent));
            };
            if stmt.owner != owner {
                return Err(Error::UnknownId(parent));
            }
        }
        let mut siblings: Vec<ElId> = self
            .children_of(parent.unwrap_or(owner))
            .iter()
            .map(|stmt| stmt.id)
            .collect();
        let position = position.min(siblings.len());
        siblings.insert(position, ElId(0));
        let id = self.alloc_id();
        siblings[position] = id;
        self.renumber(owner, parent, &siblings);
        self.statements.push(Stmt::new(id, owner, parent, position as u32, text));
        Ok(id)
    }

    /// Move a statement under `owner`/`parent` at `position`, keeping its id.
    /// Renumbering both the old and new sibling lists is what makes `children_of`
    /// order stable after an insert or a reorder.
    pub fn move_stmt(
        &mut self,
        id: ElId,
        owner: ElId,
        parent: Option<ElId>,
        position: usize,
    ) -> Result<(), Error> {
        let Some(stmt) = self.find_stmt(id).cloned() else {
            return Err(Error::UnknownId(id));
        };
        if self.find(owner).is_none() {
            return Err(Error::UnknownId(owner));
        }
        if let Some(parent) = parent {
            let Some(target) = self.find_stmt(parent) else {
                return Err(Error::UnknownId(parent));
            };
            if target.owner != owner {
                return Err(Error::UnknownId(parent));
            }
        }

        let old_parent = stmt.parent;
        let old_owner = stmt.owner;
        let old_siblings: Vec<ElId> = self
            .children_of(old_parent.unwrap_or(old_owner))
            .iter()
            .map(|sibling| sibling.id)
            .filter(|sibling| *sibling != id)
            .collect();
        self.renumber(old_owner, old_parent, &old_siblings);

        let mut target: Vec<ElId> = self
            .children_of(parent.unwrap_or(owner))
            .iter()
            .map(|sibling| sibling.id)
            .filter(|sibling| *sibling != id)
            .collect();
        let position = position.min(target.len());
        target.insert(position, id);
        self.renumber(owner, parent, &target);
        if let Some(stmt) = self.statements.iter_mut().find(|stmt| stmt.id == id) {
            stmt.owner = owner;
            stmt.parent = parent;
            stmt.index = position as u32;
        }
        Ok(())
    }

    /// Move `id` directly after `anchor` in the same body.
    pub fn reorder_after(&mut self, id: ElId, anchor: ElId) -> Result<(), Error> {
        if id == anchor {
            return Ok(());
        }
        let Some(stmt) = self.find_stmt(id) else {
            return Err(Error::UnknownId(id));
        };
        let Some(anchor_stmt) = self.find_stmt(anchor) else {
            return Err(Error::UnknownId(anchor));
        };
        let owner = stmt.owner;
        if anchor_stmt.owner != owner {
            return Err(Error::UnknownId(anchor));
        }
        let parent = anchor_stmt.parent;
        let position = self
            .children_of(parent.unwrap_or(owner))
            .iter()
            .position(|sibling| sibling.id == anchor)
            .map(|index| index + 1)
            .ok_or(Error::UnknownId(anchor))?;
        self.move_stmt(id, owner, parent, position)
    }

    fn renumber(&mut self, owner: ElId, parent: Option<ElId>, ids: &[ElId]) {
        for (index, id) in ids.iter().enumerate() {
            if let Some(stmt) = self.statements.iter_mut().find(|stmt| stmt.id == *id) {
                stmt.owner = owner;
                stmt.parent = parent;
                stmt.index = index as u32;
            }
        }
    }
}
