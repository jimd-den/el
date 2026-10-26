// `el show`: one element, by permanent id only. The fields are the catalog's
// plus three derived views — the rendered declaration, the source slice, and the
// journal entries that mention it — so a caller can answer "what is this" without
// opening the file it lives in.

use super::View;
use self::context::{doc_of, mentions, owning_sequence, source_of};
use self::detail::{entry_json, statement_json, statement_lines};
use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::render::{Projection, rust};
use crate::core::text::json::Json;

mod context;
mod detail;

pub struct Options {
    pub statements: bool,
    pub source: bool,
    pub history: bool,
}

pub fn show(
    catalog: &Catalog,
    projection: &Projection,
    entries: &[Entry],
    element: &Element,
    options: &Options,
) -> View {
    let alias = element.alias.as_ref().map(|alias| alias.to_string());
    let doc = doc_of(catalog, element);
    let sequence = owning_sequence(entries, element);
    let ids: Vec<String> = catalog
        .children_of(element.id)
        .iter()
        .map(|stmt| stmt.id.to_string())
        .collect();
    let signature = rust::declaration(element);

    let mut lines = vec![
        format!("id          {}", element.id),
        format!("kind        {}", element.kind),
        format!("alias       {}", alias.as_deref().unwrap_or("-")),
        format!("module      {}", element.module),
        format!("visibility  {}", element.visibility.as_str()),
        format!("signature   {signature}"),
        format!("doc         {}", doc.as_deref().unwrap_or("-")),
        format!("mode        {}", element.mode.as_str()),
        format!("sequence    {}", sequence.map_or("-".to_string(), |seq| seq.to_string())),
        format!(
            "statements  {}",
            if ids.is_empty() { "-".to_string() } else { ids.join(" ") }
        ),
    ];

    let mut fields = vec![
        ("id".to_string(), Json::Str(element.id.to_string())),
        ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
        ("alias".to_string(), alias.map_or(Json::Null, Json::Str)),
        ("module".to_string(), Json::Str(element.module.clone())),
        ("visibility".to_string(), Json::Str(element.visibility.as_str().to_string())),
        ("signature".to_string(), Json::Str(signature)),
        ("doc".to_string(), doc.map_or(Json::Null, Json::Str)),
        ("mode".to_string(), Json::Str(element.mode.as_str().to_string())),
        ("sequence".to_string(), sequence.map_or(Json::Null, |seq| Json::Number(seq.to_string()))),
        (
            "statements".to_string(),
            Json::Array(ids.into_iter().map(Json::Str).collect()),
        ),
    ];

    if options.statements {
        lines.push("statements:".to_string());
        statement_lines(catalog, element.id, 2, &mut lines);
        fields.push((
            "statement_tree".to_string(),
            statement_json(catalog, element.id),
        ));
    }
    if options.source {
        let source = source_of(projection, element);
        lines.push("source:".to_string());
        if let Some(text) = &source {
            for line in text.lines() {
                lines.push(format!("    {line}"));
            }
        }
        fields.push((
            "source".to_string(),
            source.map_or(Json::Null, Json::Str),
        ));
    }
    if options.history {
        let related: Vec<&Entry> = entries
            .iter()
            .rev()
            .filter(|entry| mentions(entry, element))
            .take(5)
            .collect();
        lines.push("history:".to_string());
        for entry in &related {
            lines.push(format!(
                "  {:06}  {}  {}  {}",
                entry.seq,
                entry.op.as_str(),
                entry.gate,
                entry.msg
            ));
        }
        fields.push((
            "history".to_string(),
            Json::Array(related.iter().map(|entry| entry_json(entry)).collect()),
        ));
    }

    View {
        lines,
        payload: Json::Object(fields),
    }
}

