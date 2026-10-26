// `el ls`: the catalog as a table. Ordering is the renderer's: module path, then
// id, because that is how the files will be written and how `tree` reads. A
// module's line count is its file; an item's is its span, braces included.

use super::View;
use crate::core::model::element::Element;
use crate::core::model::kind::Kind;
use crate::core::model::element::BodyMode;
use crate::core::render::Projection;
use crate::core::text::json::Json;

pub struct Filter {
    pub module: Option<String>,
    pub kind: Option<Kind>,
    pub opaque: bool,
}

pub fn ls(catalog: &crate::core::model::catalog::Catalog, projection: &Projection, filter: &Filter) -> View {
    if filter.kind == Some(Kind::Stmt) {
        return statements(catalog, projection, filter);
    }
    let mut elements: Vec<&Element> = catalog
        .elements
        .iter()
        .filter(|element| filter.module.as_deref().is_none_or(|module| module_matches(&element.module, module)))
        .filter(|element| filter.kind.is_none_or(|kind| element.kind == kind))
        .filter(|element| !filter.opaque || element.mode == BodyMode::Opaque)
        .collect();
    elements.sort_by(|a, b| (&a.module, a.id).cmp(&(&b.module, b.id)));

    let mut lines = vec![format!(
        "{:<9} {:<6} {:<28} {:<22} {:>5}  {}",
        "id", "kind", "alias", "module", "lines", "mode"
    )];
    let mut rows = Vec::new();
    for element in elements {
        let alias = element.alias.as_ref().map(|alias| alias.to_string());
        let count = line_count(element, projection);
        lines.push(format!(
            "{:<9} {:<6} {:<28} {:<22} {:>5}  {}",
            element.id,
            element.kind.as_str(),
            alias.as_deref().unwrap_or("-"),
            element.module,
            count.map_or("-".to_string(), |n| n.to_string()),
            element.mode.as_str()
        ));
        rows.push(Json::Object(vec![
            ("id".to_string(), Json::Str(element.id.to_string())),
            ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
            ("alias".to_string(), alias.map_or(Json::Null, Json::Str)),
            ("module".to_string(), Json::Str(element.module.clone())),
            ("lines".to_string(), count.map_or(Json::Null, |n| Json::Number(n.to_string()))),
            ("mode".to_string(), Json::Str(element.mode.as_str().to_string())),
        ]));
    }
    View {
        lines,
        payload: Json::Array(rows),
    }
}

fn statements(
    catalog: &crate::core::model::catalog::Catalog,
    projection: &Projection,
    filter: &Filter,
) -> View {
    let mut rows: Vec<(&crate::core::model::stmt::Stmt, &Element)> = catalog
        .statements
        .iter()
        .filter_map(|stmt| catalog.find(stmt.owner).map(|owner| (stmt, owner)))
        .filter(|(_, owner)| {
            filter
                .module
                .as_deref()
                .is_none_or(|module| module_matches(&owner.module, module))
        })
        .filter(|(_, owner)| !filter.opaque || owner.mode == BodyMode::Opaque)
        .collect();
    rows.sort_by(|(left, lo), (right, ro)| (&lo.module, left.id).cmp(&(&ro.module, right.id)));
    let mut lines = vec![format!(
        "{:<9} {:<6} {:<28} {:<22} {:>5}  {}",
        "id", "kind", "alias", "module", "lines", "mode"
    )];
    let mut out = Vec::new();
    for (stmt, owner) in rows {
        let count = projection
            .spans
            .iter()
            .find(|span| span.element == stmt.id)
            .map(|span| span.last - span.first + 1);
        lines.push(format!(
            "{:<9} {:<6} {:<28} {:<22} {:>5}  {}",
            stmt.id,
            "stmt",
            "-",
            owner.module,
            count.map_or("-".to_string(), |n| n.to_string()),
            owner.mode.as_str()
        ));
        out.push(Json::Object(vec![
            ("id".to_string(), Json::Str(stmt.id.to_string())),
            ("kind".to_string(), Json::Str("stmt".to_string())),
            ("alias".to_string(), Json::Null),
            ("module".to_string(), Json::Str(owner.module.clone())),
            ("lines".to_string(), count.map_or(Json::Null, |n| Json::Number(n.to_string()))),
            ("mode".to_string(), Json::Str(owner.mode.as_str().to_string())),
        ]));
    }
    View {
        lines,
        payload: Json::Array(out),
    }
}

fn line_count(element: &Element, projection: &Projection) -> Option<u32> {
    if element.kind == Kind::Module {
        return projection
            .file(&element.module)
            .map(|file| file.text.lines().count() as u32);
    }
    projection
        .spans
        .iter()
        .find(|span| span.element == element.id)
        .map(|span| span.last - span.first + 1)
}

/// `src/greet.rs`, `greet`, and `src` are all ways a person names a module; the
/// filter accepts each, and a directory name selects its subtree.
fn module_matches(module: &str, filter: &str) -> bool {
    if module == filter {
        return true;
    }
    let left = normalized(module);
    let right = normalized(filter);
    left == right || left.starts_with(&format!("{right}::"))
}

fn normalized(module: &str) -> String {
    let path = module.strip_prefix("src/").unwrap_or(module);
    path.strip_suffix(".rs").unwrap_or(path).replace('/', "::")
}
