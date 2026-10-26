// `el tree`: the three-level outline. The module tree comes from `render::tree`,
// so the order is the one the files will be written in; items inside a module are
// id order, and statements are `children_of` order. Nothing here reads a file.

use super::View;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::model::stmt::Stmt;
use crate::core::render::tree::Tree;
use crate::core::text::json::Json;

pub fn tree(catalog: &Catalog, crate_name: &str, statements: bool) -> View {
    let tree = Tree::of(catalog);
    let mut lines = vec![format!("crate {crate_name}")];
    let mut modules = Vec::new();

    for node in tree.modules() {
        let module = module_id(catalog, &node.path);
        lines.push(format!(
            "  {}  mod {}",
            node.path,
            module.map_or("-".to_string(), |id| id.to_string())
        ));
        let mut fields = vec![
            ("path".to_string(), Json::Str(node.path.clone())),
            ("module".to_string(), Json::Str(node.module.clone())),
            ("module_id".to_string(), module.map_or(Json::Null, |id| Json::Str(id.to_string()))),
        ];
        if statements
            && let Some(id) = module
        {
            statement_lines(catalog, id, 4, &mut lines);
            if !catalog.children_of(id).is_empty() {
                fields.push(("statement_tree".to_string(), statement_json(catalog, id)));
            }
        }

        let mut items = Vec::new();
        for id in &node.items {
            let Some(element) = catalog.find(*id) else { continue };
            lines.push(format!("    {}  {}  {}", element.id, element.kind, alias(element)));
            let mut item = vec![
                ("id".to_string(), Json::Str(element.id.to_string())),
                ("kind".to_string(), Json::Str(element.kind.as_str().to_string())),
                ("alias".to_string(), element.alias.as_ref().map_or(Json::Null, |a| Json::Str(a.to_string()))),
            ];
            if statements && !catalog.children_of(element.id).is_empty() {
                statement_lines(catalog, element.id, 6, &mut lines);
                item.push(("statement_tree".to_string(), statement_json(catalog, element.id)));
            }
            items.push(Json::Object(item));
        }
        fields.push(("items".to_string(), Json::Array(items)));
        modules.push(Json::Object(fields));
    }

    View {
        lines,
        payload: Json::Object(vec![
            ("crate".to_string(), Json::Str(crate_name.to_string())),
            ("modules".to_string(), Json::Array(modules)),
        ]),
    }
}

fn module_id(catalog: &Catalog, path: &str) -> Option<ElId> {
    catalog
        .elements
        .iter()
        .find(|element| element.kind == Kind::Module && element.module == path)
        .map(|element| element.id)
}

fn alias(element: &Element) -> String {
    element
        .alias
        .as_ref()
        .map_or("-".to_string(), |alias| alias.to_string())
}

fn statement_lines(catalog: &Catalog, parent: ElId, indent: usize, lines: &mut Vec<String>) {
    for stmt in catalog.children_of(parent) {
        for line in stmt.text.lines() {
            lines.push(format!("{}{}  {}", " ".repeat(indent), stmt.id, line.trim_end()));
        }
        statement_lines(catalog, stmt.id, indent + 2, lines);
    }
}

fn statement_json(catalog: &Catalog, parent: ElId) -> Json {
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
