// Turning one module into one file: the marker, the module's preamble, the `mod`
// lines its children need, and its items in id order.
//
// This is the only place that lays out an item's declaration, and the layout is
// deliberately mechanical: `{visibility}{keyword} {name}{head}` followed by a
// brace or a semicolon. Whether that text is *legal Rust* is the gate's verdict,
// never ours — `el` remembers characters (ADR-0008), it does not parse them.
//
// Ordering rules, all of them content-determined so that FR-13 holds: preamble,
// then `mod` lines sorted by name, then items sorted by id.

use super::super::model::catalog::Catalog;
use super::super::model::element::{Element, Visibility};
use super::super::model::kind::Kind;
use super::body;
use super::spans::Collector;
use super::style::{self, Out};
use super::tree::{Node, Tree};

/// One module's file, as text.
pub fn render_module(
    catalog: &Catalog,
    _tree: &Tree,
    node: &Node,
    spans: &mut Collector,
) -> String {
    if let Some(element) = catalog.elements.iter().find(|element| {
        element.kind == Kind::Module && element.module == node.path
    }) && element.mode == crate::core::model::element::BodyMode::Opaque
    {
        let text: String = catalog
            .children_of(element.id)
            .iter()
            .map(|stmt| stmt.text.as_str())
            .collect();
        let lines = text.lines().count() as u32;
        if lines > 0 {
            spans.push(&node.path, 1, lines, element.id);
        }
        return text;
    }
    let mut out = Out::new();
    out.bare(style::MARKER);

    if let Some(preamble) = preamble(catalog, node) {
        out.blank();
        for line in preamble.lines() {
            out.bare(line.trim_end());
        }
    }

    if !node.children.is_empty() {
        out.blank();
        for child in &node.children {
            let dir = if node.module.is_empty() {
                "src".to_string()
            } else {
                format!("src/{}", node.module.replace("::", "/"))
            };
            let child_path = format!("{}/{}.rs", dir, child);
            // A subtree holding a public item must be declared `pub mod`,
            // otherwise the item is unreachable from the crate root (FR-15).
            let has_pub = catalog.elements.iter().any(|e| {
                e.visibility == Visibility::Pub
                    && (e.module == child_path
                        || e.module.starts_with(&format!(
                            "{}/",
                            child_path.trim_end_matches(".rs")
                        )))
            });
            if has_pub {
                out.bare(&format!("pub mod {child};"));
            } else {
                out.bare(&format!("mod {child};"));
            }
        }
    }

    for id in &node.items {
        let Some(element) = catalog.find(*id) else {
            continue;
        };
        out.blank();
        render_item(catalog, node, element, &mut out, spans);
    }

    out.into_text()
}

/// The declaration text: everything before the body's `{` or the item's `;`.
pub fn declaration(element: &Element) -> String {
    match &element.name {
        Some(name) => format!(
            "{}{} {}{}",
            element.visibility.prefix(),
            element.kind.decl_keyword(),
            name,
            element.head
        ),
        None => {
            let space = if element.head.is_empty() { "" } else { " " };
            format!(
                "{}{}{}{}",
                element.visibility.prefix(),
                element.kind.decl_keyword(),
                space,
                element.head
            )
        }
    }
}

fn render_item(
    catalog: &Catalog,
    node: &Node,
    element: &Element,
    out: &mut Out,
    spans: &mut Collector,
) {
    let first = out.line();
    let declaration = declaration(element);

    // The attribute is the renderer's, not the signature's: a sig starting
    // with `#[test]` is refused, and `declaration()` stays single-line so
    // `show`, `explain` and `diff` keep working unchanged. The span starts
    // before this line, so `show --source` includes it.
    if element.kind == Kind::Test {
        out.line_at(0, "#[test]");
    }

    if element.kind.takes_block() {
        out.line_at(0, &format!("{declaration} {{"));
        body::render(catalog, node, element, out, spans);
        out.line_at(0, "}");
    } else {
        out.line_at(0, &format!("{declaration};"));
    }

    spans.push(&node.path, first, out.line() - 1, element.id);
}

/// The module element's statements, joined — the `M <path> preamble` record.
/// Returns `None` when the module declares no preamble.
fn preamble(catalog: &Catalog, node: &Node) -> Option<String> {
    let module = catalog
        .elements
        .iter()
        .find(|element| element.kind == Kind::Module && element.module == node.path)?;
    let statements = catalog.children_of(module.id);
    if statements.is_empty() {
        return None;
    }
    let text: String = statements.iter().map(|stmt| stmt.text.as_str()).collect();

    Some(text.trim_end().to_string())
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod rust_tests;
