// The element/statement view of a diff. It answers "which id changed" when both
// sequences have a catalog snapshot; a missing snapshot falls back to the file
// diff in the parent module.

use super::lines_json;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;
use crate::core::model::stmt::Stmt;
use crate::core::render::rust;
use crate::core::text::diff::{self, Line};
use crate::core::text::json::Json;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) enum Change {
    Element { op: &'static str, id: ElId, kind: String, alias: Option<String>, lines: Vec<Line> },
    Statement { op: &'static str, id: ElId, owner: ElId, lines: Vec<Line> },
}

pub(super) fn element_changes(left: &Catalog, right: &Catalog) -> Vec<Change> {
    let left_elements: BTreeMap<ElId, &Element> = left.elements.iter().map(|e| (e.id, e)).collect();
    let right_elements: BTreeMap<ElId, &Element> = right.elements.iter().map(|e| (e.id, e)).collect();
    let left_stmts: BTreeMap<ElId, &Stmt> = left.statements.iter().map(|s| (s.id, s)).collect();
    let right_stmts: BTreeMap<ElId, &Stmt> = right.statements.iter().map(|s| (s.id, s)).collect();
    let mut out = Vec::new();

    for id in union(left_elements.keys(), right_elements.keys()) {
        let before = left_elements.get(&id).map(|e| rust::declaration(e));
        let after = right_elements.get(&id).map(|e| rust::declaration(e));
        if let Some(change) = element_change(id, before, after, right_elements.get(&id)) {
            out.push(change);
        }
    }
    for id in union(left_stmts.keys(), right_stmts.keys()) {
        let before = left_stmts.get(&id).map(|s| s.text.clone());
        let after = right_stmts.get(&id).map(|s| s.text.clone());
        if let Some(change) = statement_change(id, before, after, right_stmts.get(&id).or(left_stmts.get(&id))) {
            out.push(change);
        }
    }
    out
}

fn element_change(id: ElId, before: Option<String>, after: Option<String>, right: Option<&&Element>) -> Option<Change> {
    if before == after {
        return None;
    }
    let (op, lines) = edit_lines(before, after);
    Some(Change::Element {
        op,
        id,
        kind: right.map(|e| e.kind.as_str().to_string()).unwrap_or_default(),
        alias: right.and_then(|e| e.alias.as_ref().map(|a| a.to_string())),
        lines,
    })
}

fn statement_change(id: ElId, before: Option<String>, after: Option<String>, right: Option<&&Stmt>) -> Option<Change> {
    if before == after {
        return None;
    }
    let owner = right.map(|s| s.owner).unwrap_or(ElId(0));
    let (op, lines) = edit_lines(before, after);
    Some(Change::Statement { op, id, owner, lines })
}

fn edit_lines(before: Option<String>, after: Option<String>) -> (&'static str, Vec<Line>) {
    match (before, after) {
        (Some(old), Some(new)) => ("changed", diff::lines(&old, &new)),
        (Some(old), None) => ("removed", old.lines().map(|l| Line::Remove(l.to_string())).collect()),
        (None, Some(new)) => ("added", new.lines().map(|l| Line::Add(l.to_string())).collect()),
        (None, None) => ("changed", Vec::new()),
    }
}

fn union<'a>(left: impl Iterator<Item = &'a ElId>, right: impl Iterator<Item = &'a ElId>) -> Vec<ElId> {
    let mut ids: Vec<ElId> = left.chain(right).copied().collect();
    ids.sort();
    ids.dedup();
    ids
}

pub(super) fn element_lines(change: &Change) -> Vec<String> {
    match change {
        Change::Element { op, id, kind, alias, lines } => {
            let mut out = vec![format!("{id}  {op}  {kind}  {}", alias.as_deref().unwrap_or("-"))];
            out.extend(lines.iter().map(line_text));
            out
        }
        Change::Statement { op, id, owner, lines } => {
            let mut out = vec![format!("{id}  {op}  in {owner}")];
            out.extend(lines.iter().map(line_text));
            out
        }
    }
}

pub(super) fn line_text(line: &Line) -> String {
    match line {
        Line::Same(text) => format!("  {text}"),
        Line::Remove(text) => format!("- {text}"),
        Line::Add(text) => format!("+ {text}"),
    }
}

pub(super) fn change_json(change: &Change) -> Json {
    match change {
        Change::Element { op, id, kind, alias, lines } => Json::Object(vec![
            ("op".to_string(), Json::Str(op.to_string())),
            ("id".to_string(), Json::Str(id.to_string())),
            ("kind".to_string(), Json::Str(kind.clone())),
            ("alias".to_string(), alias.clone().map_or(Json::Null, Json::Str)),
            ("lines".to_string(), lines_json(lines)),
        ]),
        Change::Statement { op, id, owner, lines } => Json::Object(vec![
            ("op".to_string(), Json::Str(op.to_string())),
            ("id".to_string(), Json::Str(id.to_string())),
            ("owner".to_string(), Json::Str(owner.to_string())),
            ("lines".to_string(), lines_json(lines)),
        ]),
    }
}

