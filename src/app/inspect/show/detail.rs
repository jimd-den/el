// The statement tree and journal-entry shapes `show` and `tree` both need. They
// are the JSON spellings of the catalog's parent links, kept apart from the field
// assembly so a change to one never moves the other.

use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;
use crate::core::model::id::ElId;
use crate::core::model::stmt::Stmt;
use crate::core::text::json::Json;

pub(super) fn statement_lines(catalog: &Catalog, parent: ElId, indent: usize, lines: &mut Vec<String>) {
    for stmt in catalog.children_of(parent) {
        for line in stmt.text.lines() {
            lines.push(format!("{}{}  {}", " ".repeat(indent), stmt.id, line.trim_end()));
        }
        statement_lines(catalog, stmt.id, indent + 2, lines);
    }
}

pub(super) fn statement_json(catalog: &Catalog, parent: ElId) -> Json {
    Json::Array(
        catalog
            .children_of(parent)
            .into_iter()
            .map(|stmt| statement_node(catalog, stmt))
            .collect(),
    )
}

fn statement_node(catalog: &Catalog, stmt: &Stmt) -> Json {
    Json::Object(vec![
        ("id".to_string(), Json::Str(stmt.id.to_string())),
        ("text".to_string(), Json::Str(stmt.text.clone())),
        ("children".to_string(), statement_json(catalog, stmt.id)),
    ])
}

pub(super) fn entry_json(entry: &Entry) -> Json {
    Json::Object(vec![
        ("seq".to_string(), Json::Number(entry.seq.to_string())),
        ("op".to_string(), Json::Str(entry.op.as_str().to_string())),
        ("gate".to_string(), Json::Str(entry.gate.clone())),
        ("msg".to_string(), Json::Str(entry.msg.clone())),
    ])
}
