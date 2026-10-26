// The one-line spellings `show --terse` prints. One line per element, one
// line per statement (plus indented continuations for a multi-line statement
// text), so an agent can hold a whole function in a few dozen bytes. The JSON
// payload is untouched: terse is a rendering of the lines half only, and
// `--source` / `--history` sections keep their existing shape.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;
use crate::core::model::stmt::Stmt;

pub(super) fn element_line(element: &Element, signature: &str) -> String {
    format!(
        "{} {} {} {} {}",
        element.id,
        element.kind,
        element.alias.as_ref().map_or("-".to_string(), |alias| alias.to_string()),
        element.module,
        signature
    )
}

pub(super) fn stmt_header(stmt: &Stmt, lines: &mut Vec<String>) {
    let parent = stmt.parent.map_or("-".to_string(), |parent| parent.to_string());
    push_text(
        lines,
        &format!("{} stmt owner {} parent {} ", stmt.id, stmt.owner, parent),
        "  ",
        &stmt.text,
    );
}

pub(super) fn statement_lines(catalog: &Catalog, parent: ElId, indent: usize, lines: &mut Vec<String>) {
    for stmt in catalog.children_of(parent) {
        let pad = " ".repeat(indent);
        push_text(lines, &format!("{pad}{} ", stmt.id), &" ".repeat(indent + 2), &stmt.text);
        statement_lines(catalog, stmt.id, indent + 2, lines);
    }
}

fn push_text(lines: &mut Vec<String>, first_prefix: &str, cont_indent: &str, text: &str) {
    let mut text_lines = text.lines();
    match text_lines.next() {
        Some(first) => lines.push(format!("{first_prefix}{}", first.trim_end())),
        None => lines.push(first_prefix.trim_end().to_string()),
    }
    for line in text_lines {
        lines.push(format!("{cont_indent}{}", line.trim_end()));
    }
}
