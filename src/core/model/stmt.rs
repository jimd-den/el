// A statement is the smallest thing `el` edits: one line of a body, with its own
// permanent id, an owner (the element whose body it belongs to), a position
// among its siblings, and an optional parent statement.
//
// Nesting is a parent link rather than a tree of owned values, so a subtree can
// be reordered by changing `index` and a statement can be moved without cloning
// its children. FR-62: to any depth, every statement addressable.

use super::id::ElId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub id: ElId,
    pub owner: ElId,
    pub parent: Option<ElId>,
    pub index: u32,
    pub text: String,
}

impl Stmt {
    /// Build a statement for an id the catalog has already issued.
    pub fn new(id: ElId, owner: ElId, parent: Option<ElId>, index: u32, text: &str) -> Self {
        Self {
            id,
            owner,
            parent,
            index,
            text: text.to_string(),
        }
    }

    /// Whether this statement is a direct child of `parent`.
    pub fn is_child_of(&self, parent: ElId) -> bool {
        self.parent == Some(parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_statement_keeps_its_text_verbatim() {
        let text = "let s = \"a\\tb\\n\";";

        assert_eq!(Stmt::new(ElId(3), ElId(1), None, 0, text).text, text);
    }

    #[test]
    fn a_statement_knows_its_parent() {
        let stmt = Stmt::new(ElId(4), ElId(1), Some(ElId(3)), 0, "b();");

        assert!(stmt.is_child_of(ElId(3)));
        assert!(!stmt.is_child_of(ElId(1)));
    }
}
