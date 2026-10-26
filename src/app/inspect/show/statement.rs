// `show` for a statement: the same envelope, addressed by the statement id. The
// owner is the element that carries the body, so module/sequence/mode come from
// it; the text and children are the statement's own.

use super::super::View;
use super::context::{mentions, owning_sequence};
use super::detail::{entry_json, statement_json, statement_lines};
use super::Options;
use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;
use crate::core::model::stmt::Stmt;
use crate::core::render::Projection;
use crate::core::text::json::Json;

pub fn statement(
    catalog: &Catalog,
    projection: &Projection,
    entries: &[Entry],
    stmt: &Stmt,
    options: &Options,
) -> View {
    let owner = catalog.find(stmt.owner);
    let module = owner.map(|element| element.module.clone()).unwrap_or_default();
    let sequence = owner.and_then(|element| owning_sequence(entries, element));
    let ids: Vec<String> = catalog
        .children_of(stmt.id)
        .iter()
        .map(|child| child.id.to_string())
        .collect();
    let mut lines = vec![
        format!("id          {}", stmt.id),
        "kind        stmt".to_string(),
        format!("owner       {}", stmt.owner),
        format!("parent      {}", stmt.parent.map_or("-".to_string(), |parent| parent.to_string())),
        format!("index       {}", stmt.index),
        format!("module      {module}"),
        format!("sequence    {}", sequence.map_or("-".to_string(), |seq| seq.to_string())),
        format!("text        {}", stmt.text.lines().next().unwrap_or("")),
        format!("statements  {}", if ids.is_empty() { "-".to_string() } else { ids.join(" ") }),
    ];
    let mut fields = vec![
        ("id".to_string(), Json::Str(stmt.id.to_string())),
        ("kind".to_string(), Json::Str("stmt".to_string())),
        ("owner".to_string(), Json::Str(stmt.owner.to_string())),
        ("parent".to_string(), stmt.parent.map_or(Json::Null, |parent| Json::Str(parent.to_string()))),
        ("index".to_string(), Json::Number(stmt.index.to_string())),
        ("module".to_string(), Json::Str(module)),
        ("sequence".to_string(), sequence.map_or(Json::Null, |seq| Json::Number(seq.to_string()))),
        ("text".to_string(), Json::Str(stmt.text.clone())),
        ("statements".to_string(), Json::Array(ids.into_iter().map(Json::Str).collect())),
    ];
    if options.statements {
        lines.push("statements:".to_string());
        statement_lines(catalog, stmt.id, 2, &mut lines);
        fields.push(("statement_tree".to_string(), statement_json(catalog, stmt.id)));
    }
    if options.source {
        let source = stmt_source(projection, stmt);
        lines.push("source:".to_string());
        if let Some(text) = &source {
            for line in text.lines() {
                lines.push(format!("    {line}"));
            }
        }
        fields.push(("source".to_string(), source.map_or(Json::Null, Json::Str)));
    }
    if options.history {
        let related: Vec<&Entry> = entries
            .iter()
            .rev()
            .filter(|entry| {
                entry.target == stmt.id.to_string()
                    || owner.is_some_and(|element| mentions(entry, element))
            })
            .take(5)
            .collect();
        lines.push("history:".to_string());
        for entry in &related {
            lines.push(format!("  {:06}  {}  {}  {}", entry.seq, entry.op.as_str(), entry.gate, entry.msg));
        }
        fields.push(("history".to_string(), Json::Array(related.iter().map(|entry| entry_json(entry)).collect())));
    }
    View { lines, payload: Json::Object(fields) }
}

fn stmt_source(projection: &Projection, stmt: &Stmt) -> Option<String> {
    let span = projection.spans.iter().find(|span| span.element == stmt.id)?;
    let file = projection.file(&span.path)?;
    let lines: Vec<&str> = file.text.lines().collect();
    let start = span.first.saturating_sub(1) as usize;
    let end = span.last as usize;
    lines.get(start..end).map(|slice| slice.join("\n"))
}

