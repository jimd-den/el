// `el explain`: why an element exists, in one page. It answers from the catalog,
// never from a running language server, and says so — `rust-analyzer` is not built
// (M10), and a page that implied it had looked would be lying.
//
// Callers and tests are a whole-word scan over statement text, so they are
// references a reader can check, not name resolution. That is the honest limit:
// the answer is cheaper and always available; `refs` will be the semantic one.

mod scan;

use self::scan::{callers, tests, Caller};
use super::{show, View};
use crate::core::history::entry::Entry;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::kind::Kind;
use crate::core::render::rust;
use crate::core::text::json::Json;

pub fn explain(catalog: &Catalog, entries: &[Entry], element: &Element) -> View {
    let signature = rust::declaration(element);
    let preamble = module_preamble(catalog, element);
    let doc = show::doc_of(catalog, element);
    let callers = callers(catalog, element);
    let tests = tests(catalog, element);
    let history: Vec<&Entry> = entries
        .iter()
        .rev()
        .filter(|entry| show::mentions(entry, element))
        .take(5)
        .collect();

    let mut lines = vec![
        format!("id          {}", element.id),
        format!("kind        {}", element.kind),
        format!("alias       {}", alias(element)),
        format!("module      {}", element.module),
        format!("signature   {signature}"),
    ];
    section(&mut lines, "preamble", preamble.as_deref());
    section(&mut lines, "doc", doc.as_deref());
    lines.push(format!("callers     {}", callers.len()));
    for caller in &callers {
        lines.push(format!("  {}  {}  {}", caller.id, caller.alias, caller.text));
    }
    lines.push(format!("tests       {}", tests.len()));
    for test in &tests {
        lines.push(format!("  {}  {}", test.id, test.alias));
    }
    lines.push(format!("history     {}", history.len()));
    for entry in &history {
        lines.push(format!(
            "  {:06}  {}  {}  {}",
            entry.seq,
            entry.op.as_str(),
            entry.gate,
            entry.msg
        ));
    }
    lines.push("source      catalog (rust-analyzer not built)".to_string());

    let payload = Json::Object(vec![
        ("id".to_string(), Json::Str(element.id.to_string())),
        ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
        ("alias".to_string(), Json::Str(alias(element))),
        ("module".to_string(), Json::Str(element.module.clone())),
        ("signature".to_string(), Json::Str(signature)),
        ("preamble".to_string(), preamble.map_or(Json::Null, Json::Str)),
        ("doc".to_string(), doc.map_or(Json::Null, Json::Str)),
        ("callers".to_string(), Json::Array(callers.iter().map(caller_json).collect())),
        ("tests".to_string(), Json::Array(tests.iter().map(test_json).collect())),
        (
            "history".to_string(),
            Json::Array(
                history
                    .iter()
                    .map(|entry| {
                        Json::Object(vec![
                            ("seq".to_string(), Json::Number(entry.seq.to_string())),
                            ("op".to_string(), Json::Str(entry.op.as_str().to_string())),
                            ("gate".to_string(), Json::Str(entry.gate.clone())),
                            ("msg".to_string(), Json::Str(entry.msg.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("source".to_string(), Json::Str("catalog".to_string())),
    ]);

    View { lines, payload }
}

fn module_preamble(catalog: &Catalog, element: &Element) -> Option<String> {
    let module = catalog
        .elements
        .iter()
        .find(|candidate| {
            candidate.kind == Kind::Module && candidate.module == element.module
        })?;
    show::doc_of(catalog, module)
}

pub(super) fn alias(element: &Element) -> String {
    element
        .alias
        .as_ref()
        .map(|alias| alias.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn section(lines: &mut Vec<String>, name: &str, text: Option<&str>) {
    match text {
        None => lines.push(format!("{name:<12}-")),
        Some(text) => {
            for (index, line) in text.lines().enumerate() {
                let label = if index == 0 { name } else { "" };
                lines.push(format!("{label:<12}{line}"));
            }
        }
    }
}

fn caller_json(caller: &Caller) -> Json {
    Json::Object(vec![
        ("id".to_string(), Json::Str(caller.id.to_string())),
        ("alias".to_string(), Json::Str(caller.alias.clone())),
        ("text".to_string(), Json::Str(caller.text.clone())),
    ])
}

fn test_json(test: &Caller) -> Json {
    Json::Object(vec![
        ("id".to_string(), Json::Str(test.id.to_string())),
        ("alias".to_string(), Json::Str(test.alias.clone())),
    ])
}
