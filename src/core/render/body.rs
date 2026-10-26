// A body, from statements back to text. Two modes, and the difference is the
// whole point of `BodyMode`:
//
//   Split   every statement is one line, indented by its nesting depth. Leading
//           whitespace is structural, so it is recomputed; the interior of the
//           statement is never touched (FR-67).
//   Opaque  the stored text is written as it was read. Re-indenting it would
//           change content the writer of the blob owns, which is exactly what a
//           body containing a multi-line raw string needs.
//
// It never decides whether a statement is legal Rust, and it never splits: the
// split happened upstream (`text::split`), and rendering a body is the inverse.

use super::super::model::catalog::Catalog;
use super::super::model::element::{BodyMode, Element};
use super::super::model::id::ElId;
use super::spans::Collector;
use super::style::Out;
use super::tree::Node;

/// Write `element`'s body, between its opening brace and its closing one.
pub fn render(
    catalog: &Catalog,
    node: &Node,
    element: &Element,
    out: &mut Out,
    spans: &mut Collector,
) {
    match element.mode {
        BodyMode::Split => walk(catalog, node, element.id, 1, out, spans),
        BodyMode::Opaque => opaque(catalog, node, element.id, out, spans),
    }
}

/// Statements in source order, each followed by its own children, one level
/// deeper.
fn walk(
    catalog: &Catalog,
    node: &Node,
    parent: ElId,
    depth: usize,
    out: &mut Out,
    spans: &mut Collector,
) {
    for stmt in catalog.children_of(parent) {
        let first = out.line();
        lines(out, depth, &stmt.text);
        walk(catalog, node, stmt.id, depth + 1, out, spans);
        if opens_block(&stmt.text) {
            out.line_at(depth, "}");
        }
        spans.push(&node.path, first, out.line() - 1, stmt.id);
    }
}

/// Whether the statement's text opens a block that its children fill.
///
/// The model records the nesting, not the punctuation (FR-62): a statement owns
/// children exactly when it opened a block. Ending in `{` is how that is visible
/// in the text, and it is reliable *because* `text::split` decided where the
/// statement ended — this reads a decision, it does not make one.
fn opens_block(text: &str) -> bool {
    text.trim_end().ends_with('{')
}

/// One statement's text, one output line per stored line. The first line loses
/// its leading whitespace (structural); later lines keep their bytes except for
/// leading whitespace, which is indentation by another name.
fn lines(out: &mut Out, depth: usize, text: &str) {
    let trimmed = text.trim_end();
    let mut wrote = false;

    for (index, line) in trimmed.lines().enumerate() {
        let body = if index == 0 {
            line.trim()
        } else {
            line.trim_start().trim_end()
        };
        if body.is_empty() {
            out.bare("");
        } else {
            out.line_at(depth, body);
        }
        wrote = true;
    }
    if !wrote {
        out.bare("");
    }
}

/// An opaque body: the blob exactly as stored, its first line indented so the
/// file still looks like Rust.
fn opaque(catalog: &Catalog, node: &Node, owner: ElId, out: &mut Out, spans: &mut Collector) {
    for stmt in catalog.children_of(owner) {
        let first = out.line();
        out.verbatim(stmt.text.trim_end(), 1);
        if out.line() > first {
            spans.push(&node.path, first, out.line() - 1, stmt.id);
        }
    }
}

#[cfg(test)]
#[path = "body_tests.rs"]
mod body_tests;
