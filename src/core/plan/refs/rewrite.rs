// Rewriting references once a name changes: `rename` and the statement/head pairs it
// touches. Split from `refs`, which finds references, because the two answer
// different questions — "where is this name used" is an index, "what becomes of
// those uses" is a decision — and only this half knows the element's kind.

use super::super::Rewrite;
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::text::ident;

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

/// Whether a hit is the renamed name itself, not a path segment or a member of
/// something else. A catalog cannot resolve names, but the kind settles the
/// shapes a bare identifier has in Rust.
///
/// A module is only a qualifier (`greet::x`), so a bare `use greet;` is left to
/// the gate. A function, const or static skips a qualifier (`m::f`) but keeps a
/// turbofish (`f::<u8>`). A type keeps its associated paths (`Task::new`). Every
/// kind skips member access (`.name`).
fn is_own_name(text: &str, offset: usize, old: &str, kind: Kind) -> bool {
    let after = text[offset + old.len()..].trim_start();
    let before = text[..offset].trim_end();
    if before.ends_with('.') && !before.ends_with("..") {
        return false;
    }
    match kind {
        Kind::Module => after.starts_with("::"),
        Kind::Function | Kind::Const | Kind::Static => {
            !after.starts_with("::") || after.starts_with("::<")
        }
        _ => true,
    }
}

/// `rewrite`, but only at the sites `is_own_name` accepts. `None` when `text`
/// cannot be scanned, like `rewrite`.
fn rewrite_own(text: &str, old: &str, new: &str, kind: Kind) -> Option<String> {
    let hits = ident::occurrences(text, old)?;
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;

    for hit in hits {
        if !is_own_name(text, hit, old, kind) {
            continue;
        }
        out.push_str(&text[cursor..hit]);
        out.push_str(new);
        cursor = hit + old.len();
    }
    out.push_str(&text[cursor..]);

    Some(out)
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
    let Some(kind) = catalog.find(renamed).map(|element| element.kind) else {
        return Vec::new();
    };
    let mut changed = Vec::new();

    for stmt in &mut catalog.statements {
        let Some(after) = rewrite_own(&stmt.text, old, &new, kind) else {
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
        let Some(after) = rewrite_own(&element.head, old, &new, kind) else {
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

