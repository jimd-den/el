// The derived fields `show` adds to the catalog: the doc text a reader would see,
// the sequence that last mentioned the element, and the source slice the renderer
// committed. They are separate from the field assembly because each has its own
// rule and its own failure mode.

use crate::core::history::entry::{Entry, Op};
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::kind::Kind;
use crate::core::render::Projection;

pub(crate) fn doc_of(catalog: &Catalog, element: &Element) -> Option<String> {
    let statements: Vec<String> = catalog
        .children_of(element.id)
        .into_iter()
        .filter_map(|stmt| {
            if element.kind == Kind::Module {
                return Some(stmt.text.trim_end().to_string());
            }
            let doc: Vec<&str> = stmt
                .text
                .lines()
                .take_while(|line| {
                    let line = line.trim_start();
                    line.starts_with("///") || line.starts_with("//!")
                })
                .collect();
            (!doc.is_empty()).then(|| doc.join("\n"))
        })
        .collect();
    if statements.is_empty() {
        None
    } else {
        Some(statements.join("\n"))
    }
}

pub(super) fn owning_sequence(entries: &[Entry], element: &Element) -> Option<u64> {
    entries
        .iter()
        .rev()
        .find(|entry| mentions(entry, element))
        .map(|entry| entry.seq)
}

pub(crate) fn mentions(entry: &Entry, element: &Element) -> bool {
    let id = element.id.to_string();
    if entry.target == id || entry.msg.contains(&id) {
        return true;
    }
    if let Some(name) = &element.name {
        // An `add` entry names its kind and its subject: `fn greet`, `mod
        // src/greet.rs`. Requiring the kind keeps the function `greet` from
        // owning the module `greet`, which a bare word match would do.
        let same_kind = words(&entry.target).next().and_then(Kind::parse) == Some(element.kind);
        if same_kind && words(&entry.target).any(|word| word == name.as_str()) {
            return true;
        }
    }
    if element.kind == Kind::Module
        && (entry.target.contains(&element.module) || entry.msg.contains(&element.module))
    {
        return true;
    }
    element.kind == Kind::Module
        && entry.op == Op::Add
        && entry.msg.starts_with("created")
        && (element.module == "src/main.rs" || element.module == "src/lib.rs")
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
}

pub(super) fn source_of(projection: &Projection, element: &Element) -> Option<String> {
    if element.kind == Kind::Module {
        return projection.file(&element.module).map(|file| file.text.clone());
    }
    let span = projection
        .spans
        .iter()
        .find(|span| span.element == element.id)?;
    let file = projection.file(&span.path)?;
    let lines: Vec<&str> = file.text.lines().collect();
    let start = span.first.saturating_sub(1) as usize;
    let end = span.last as usize;
    lines.get(start..end).map(|slice| slice.join("\n"))
}
