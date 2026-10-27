// `el find`: named elements by name, alias, or module path. It answers
// from the catalog only, like the other read verbs, and prints outline
// lines — no bodies — so an agent loop can resolve an id without paying
// for `tree`'s whole-crate dump. A query matches exactly: the element's
// name, its full alias (`greet::fn/greet`), or its module path
// (`greet::greet`). No match is the caller's to report, not an empty
// success dressed up.

use super::View;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::text::json::Json;

pub fn find(catalog: &Catalog, query: &str) -> View {
    let mut elements: Vec<&Element> = catalog
        .elements
        .iter()
        .filter(|element| matches(element, query))
        .collect();
    elements.sort_by(|a, b| (&a.module, a.id).cmp(&(&b.module, b.id)));

    let mut lines = Vec::new();
    let mut rows = Vec::new();
    for element in elements {
        let alias = element.alias.as_ref().map(|alias| alias.to_string());
        lines.push(format!(
            "{} {}",
            element.id,
            alias.as_deref().unwrap_or("-")
        ));
        rows.push(Json::Object(vec![
            ("id".to_string(), Json::Str(element.id.to_string())),
            ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
            ("alias".to_string(), alias.map_or(Json::Null, Json::Str)),
            ("module".to_string(), Json::Str(element.module.clone())),
        ]));
    }
    View {
        lines,
        payload: Json::Array(rows),
    }
}

fn matches(element: &Element, query: &str) -> bool {
    if element.name.as_deref() == Some(query) {
        return true;
    }
    if element.alias.as_ref().is_some_and(|alias| alias.as_str() == query) {
        return true;
    }
    element.name.as_deref().is_some_and(|name| path_form(&element.module, name) == query)
}

/// The `module::name` form, normalizing the file path the way
/// `Alias::derive` does: `src/a/b.rs` becomes `a::b`.
fn path_form(module: &str, name: &str) -> String {
    let trimmed = module.strip_prefix("src/").unwrap_or(module);
    let trimmed = trimmed.strip_suffix(".rs").unwrap_or(trimmed);
    format!("{}::{name}", trimmed.replace('/', "::"))
}
