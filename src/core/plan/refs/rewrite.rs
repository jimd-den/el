// Rewriting references once a name changes: `rename` and the statement/head pairs it
// touches. Split from `refs`, which finds references, because the two answer
// different questions — "where is this name used" is an index, "what becomes of
// those uses" is a decision — and only this half knows the element's kind.

use super::super::Rewrite;
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;
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
/// something else. A catalog cannot resolve names, but the kind settles the two
/// shapes a bare identifier has in Rust: `greet::x` names a module, `.greet()`
/// names a method or field.
///
/// Consequence: renaming a module rewrites only its qualifier sites (`greet::`),
/// so a bare `use greet;` is left alone and the gate refuses the change rather
/// than the rename silently claiming it was complete.
fn is_own_name(text: &str, offset: usize, old: &str, module: bool) -> bool {
    let after = text[offset + old.len()..].trim_start();
    let before = text[..offset].trim_end();
    if module {
        return after.starts_with("::");
    }
    !after.starts_with("::") && !(before.ends_with('.') && !before.ends_with(".."))
}

/// `rewrite`, but only at the sites `is_own_name` accepts. `None` when `text`
/// cannot be scanned, like `rewrite`.
fn rewrite_own(text: &str, old: &str, new: &str, module: bool) -> Option<String> {
    let hits = ident::occurrences(text, old)?;
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;

    for hit in hits {
        if !is_own_name(text, hit, old, module) {
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
    let module = catalog
        .find(renamed)
        .is_some_and(|element| element.kind == crate::core::model::kind::Kind::Module);
    let mut changed = Vec::new();

    for stmt in &mut catalog.statements {
        let Some(after) = rewrite_own(&stmt.text, old, &new, module) else {
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
        let Some(after) = rewrite_own(&element.head, old, &new, module) else {
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

