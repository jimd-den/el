// Renaming and removing both need the same answer: *where is this name used as
// code?* Not "where does the word appear" — a `"apply"` in a log message is not a
// call, and neither is `apply` in a comment or `applier` next to it. Answering
// that with `str::replace` is how a renamer corrupts a program, so the answer comes
// from `text::ident`, which knows where code starts.
//
// A reference is a statement's text or an element's head plus a byte offset into
// it. Element *names* are not sites: the name is identity, and it is what a rename
// changes, not something it rewrites.
//
// It deliberately does not resolve what a name refers to — `a::b` is two
// references to two names, and which one is the module is the compiler's business.
// It also refuses wholesale: one unscannable body means `None`, because a rename
// that rewrites most of the references is worse than one that rewrites none.

use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;
use crate::core::text::ident;

use super::Rewrite;

/// The text a reference lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// A statement's text (the body of an element, nested or not).
    Stmt(ElId),
    /// An element's head: the parameter list, type, or `impl` target.
    Head(ElId),
}

impl Site {
    /// The element id a violation should name for this site.
    pub fn element(self) -> ElId {
        match self {
            Site::Stmt(id) => id,
            Site::Head(id) => id,
        }
    }
}

/// One use of a name, and where it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub site: Site,
    /// Byte offset into the statement text or head, so a caller can rewrite in
    /// place without searching again.
    pub offset: usize,
}

/// Every code occurrence of `name`, in catalog order: statements first, then
/// heads, each in source order.
///
/// Contract: `None` when any body or head cannot be scanned (an unterminated
/// literal or comment), because then neither a rename nor a safe removal can be
/// proven.
pub fn references(catalog: &Catalog, name: &str) -> Option<Vec<Reference>> {
    let mut found = Vec::new();

    for stmt in &catalog.statements {
        for offset in ident::occurrences(&stmt.text, name)? {
            found.push(Reference {
                site: Site::Stmt(stmt.id),
                offset,
            });
        }
    }
    for element in &catalog.elements {
        for offset in ident::occurrences(&element.head, name)? {
            found.push(Reference {
                site: Site::Head(element.id),
                offset,
            });
        }
    }

    Some(found)
}

/// `text` with every whole-identifier occurrence of `old` replaced by `new`.
///
/// Contract: strings, raw strings, char literals, lifetimes and comments are
/// untouched; `None` when `text` cannot be scanned.
pub fn rewrite(text: &str, old: &str, new: &str) -> Option<String> {
    let hits = ident::occurrences(text, old)?;
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;

    for hit in hits {
        out.push_str(&text[cursor..hit]);
        out.push_str(new);
        cursor = hit + old.len();
    }
    out.push_str(&text[cursor..]);

    Some(out)
}

/// Which names are referred to at all, for a rename that has to happen before a
/// removal. Sorted and deduplicated so a caller can iterate without sorting.
pub fn names_in_use(catalog: &Catalog, names: &[String]) -> Option<Vec<String>> {
    let mut in_use: Vec<String> = names
        .iter()
        .filter(|name| references(catalog, name).is_some_and(|hits| !hits.is_empty()))
        .cloned()
        .collect();
    in_use.sort();
    in_use.dedup();
    Some(in_use)
}

/// Rewrite every code reference to `old` as the element's current name, now that
/// the rename has been applied. Returns the sites that changed, in catalog order.
///
/// The renamed element's own head is skipped: its head is a parameter list or an
/// `impl` target, so a name written there belongs to something else. An
/// unscannable body produces no rewrite here and is refused by L0 a moment later,
/// which is a better report than a partial rename.
pub fn rename_references(catalog: &mut Catalog, renamed: ElId, old: &str) -> Vec<Rewrite> {
    let Some(new) = catalog.find(renamed).and_then(|element| element.name.clone()) else {
        return Vec::new();
    };
    if new == old {
        return Vec::new();
    }
    let mut changed = Vec::new();

    for stmt in &mut catalog.statements {
        let Some(after) = rewrite(&stmt.text, old, &new) else {
            continue;
        };
        if after != stmt.text {
            let before = std::mem::replace(&mut stmt.text, after.clone());
            changed.push(Rewrite {
                site: stmt.id,
                before,
                after,
            });
        }
    }
    for element in &mut catalog.elements {
        if element.id == renamed {
            continue;
        }
        let Some(after) = rewrite(&element.head, old, &new) else {
            continue;
        };
        if after != element.head {
            let before = std::mem::replace(&mut element.head, after.clone());
            changed.push(Rewrite {
                site: element.id,
                before,
                after,
            });
        }
    }

    changed
}

#[cfg(test)]
#[path = "refs_tests.rs"]
mod refs_tests;
