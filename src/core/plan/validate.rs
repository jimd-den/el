// L0, first half: the rules a catalog must obey *as it stands*, checked after a
// change has been applied and before anything is rendered to disk. They are
// cheap, they are pure, and they run first because a model error would make the
// compiler's answer noise (GATE.md §5, FR-18).
//
// Every rule reports the element, the measured size where there is one, and the
// smallest action that would satisfy it — "your file is too long" without a seam
// is not advice (FR-73). The two rules that need a size are in `super::sizes`, and
// the three that depend on what was *asked for* are in `super::request`, so each
// file answers one question.
//
// It deliberately does not look at a single byte of the filesystem: the text is
// the projection, which is exactly what would be written.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::BodyMode;
use crate::core::plan::policy::Policy;
use crate::core::plan::violation::{Code, Violation};
use crate::core::plan::rendered;
use crate::core::render::Projection;
use crate::core::text::{balance, ident};

/// Rust's reserved and strict keywords. A name from this list cannot be a
/// declaration, so an element carrying one is unrenderable rather than merely
/// ugly. `r#`-escaped identifiers are not accepted anywhere in `el` yet, so they
/// are not subtracted here.
const KEYWORDS: [&str; 38] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where",
];

/// The eight rules about the resulting catalog, in the order GATE.md lists them.
pub fn state(catalog: &Catalog, projection: &Projection, policy: &Policy) -> Vec<Violation> {
    let mut found = Vec::new();

    duplicate_aliases(catalog, &mut found);
    unknown_refs(catalog, &mut found);
    unbalanced(catalog, &mut found);
    bad_idents(catalog, &mut found);
    rendered::checks(catalog, projection, policy, &mut found);
    forbidden(catalog, policy, &mut found);

    found
}

fn duplicate_aliases(catalog: &Catalog, out: &mut Vec<Violation>) {
    for (index, element) in catalog.elements.iter().enumerate() {
        let Some(alias) = &element.alias else { continue };
        let holder = catalog.elements[..index]
            .iter()
            .find(|other| other.alias.as_ref() == Some(alias));
        if let Some(holder) = holder {
            out.push(Violation::at(
                Code::DuplicateAlias,
                element.id,
                format!("{alias} already belongs to {}", holder.id),
            ));
        }
    }
}

fn unknown_refs(catalog: &Catalog, out: &mut Vec<Violation>) {
    for element in &catalog.elements {
        if element.module.is_empty() {
            out.push(Violation::at(
                Code::UnknownRef,
                element.id,
                "name the module this element lives in",
            ));
        }
        if let Some(parent) = element.parent
            && catalog.find(parent).is_none()
        {
            out.push(Violation::at(
                Code::UnknownRef,
                element.id,
                format!("{parent} does not exist as an owner"),
            ));
        }
    }
    for stmt in &catalog.statements {
        if catalog.find(stmt.owner).is_none() {
            out.push(Violation::at(
                Code::UnknownRef,
                stmt.id,
                format!("owner {} does not exist", stmt.owner),
            ));
        }
        if let Some(parent) = stmt.parent
            && catalog.find_stmt(parent).is_none()
        {
            out.push(Violation::at(
                Code::UnknownRef,
                stmt.id,
                format!("{parent} does not exist as a parent"),
            ));
        }
    }
}

fn unbalanced(catalog: &Catalog, out: &mut Vec<Violation>) {
    let head = catalog.elements.iter().filter_map(|element| {
        let open = balance::balanced(&element.head);
        (open != Some(true)).then_some((element.id, "head"))
    });
    // A block is a parent statement plus its children, and the renderer
    // closes it — so balance is judged over the subtree, not per statement.
    // Only roots are checked: a child is covered by its root's subtree, and
    // a statement whose parent is missing is treated as a root of its own.
    let body = catalog
        .statements
        .iter()
        .filter(|stmt| !opaque_owner(catalog, stmt.owner))
        .filter(|stmt| {
            stmt.parent.is_none()
                || stmt.parent.is_some_and(|parent| catalog.find_stmt(parent).is_none())
        })
        .filter_map(|stmt| {
            let open = balance::balanced(&subtree_text(catalog, stmt));
            (open != Some(true)).then_some((stmt.id, "text"))
        });

    for (id, part) in head.chain(body) {
        out.push(Violation::at(
            Code::Unbalanced,
            id,
            format!("the {part} opens a delimiter it never closes"),
        ));
    }
}

/// The text the renderer will write for a statement's subtree: the statement,
/// its children in source order, and the `}` the renderer emits after a block
/// opener's children. A childless opener gets no closer, so it is still
/// refused — the grammar likewise refuses a block that never gains a body.
fn subtree_text(catalog: &Catalog, stmt: &crate::core::model::stmt::Stmt) -> String {
    let mut text = stmt.text.clone();
    let children = catalog.children_of(stmt.id);
    for child in &children {
        text.push_str(&subtree_text(catalog, child));
    }
    if !children.is_empty() && stmt.text.trim_end().ends_with('{') {
        text.push('}');
    }
    text
}

fn opaque_owner(catalog: &Catalog, owner: crate::core::model::id::ElId) -> bool {
    catalog
        .find(owner)
        .is_some_and(|element| element.mode == BodyMode::Opaque)
}

fn bad_idents(catalog: &Catalog, out: &mut Vec<Violation>) {
    for element in &catalog.elements {
        let Some(name) = &element.name else { continue };
        if !is_ident(name) {
            out.push(Violation::at(
                Code::BadIdent,
                element.id,
                format!("{name:?} is a keyword or not an identifier; rename it"),
            ));
        }
    }
}

fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let starts_well = chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_');
    let continues_well = chars.all(|c| c.is_alphanumeric() || c == '_');
    starts_well && continues_well && !KEYWORDS.contains(&name)
}

fn forbidden(catalog: &Catalog, policy: &Policy, out: &mut Vec<Violation>) {
    for word in &policy.forbid {
        let sites = catalog
            .statements
            .iter()
            .map(|stmt| (stmt.id, &stmt.text))
            .chain(catalog.elements.iter().map(|element| (element.id, &element.head)));

        for (id, text) in sites {
            if ident::occurrences(text, word).is_some_and(|hits| !hits.is_empty()) {
                out.push(Violation::at(
                    Code::Forbidden,
                    id,
                    format!("`{word}` is forbidden by el.policy; rewrite it, or drop it from `forbid`"),
                ));
            }
        }
    }
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod validate_tests;
