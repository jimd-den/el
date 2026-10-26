// The `changed` list the envelope publishes: what a mutation did, in ids and
// operations. It is separated from the request builders because it reads two
// catalogs and decides nothing about the change itself.

use super::super::report::Changed;
use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;

/// What changed between two catalogs, in the envelope's words.
pub fn describe(change: &Change, before: &Catalog, after: &Catalog) -> Vec<Changed> {
    match change {
        Change::AddElement { kind, .. } => added_element(before, after, *kind),
        Change::AddStmt { .. } => added_statement(before, after),
        Change::EditElement { id, .. } | Change::EditStmt { id, .. } | Change::EditBody { id, .. } => {
            one("edit", *id, after)
        }
        Change::ReorderStmt { id, .. } => one("mv", *id, after),
        Change::MoveElement { id, .. } => one("mv", *id, after),
        Change::Rename { id, .. } => one("rename", *id, after),
        Change::Remove { id } => vec![Changed {
            op: "rm",
            id: id.to_string(),
            alias: None,
        }],
        _ => Vec::new(),
    }
}

fn one(op: &'static str, id: ElId, after: &Catalog) -> Vec<Changed> {
    vec![Changed {
        op,
        id: id.to_string(),
        alias: after
            .find(id)
            .and_then(|element| element.alias.as_ref().map(|alias| alias.to_string())),
    }]
}

fn added_element(before: &Catalog, after: &Catalog, kind: Kind) -> Vec<Changed> {
    let Some(element) = after
        .elements
        .iter()
        .find(|element| element.kind == kind && before.find(element.id).is_none())
    else {
        return Vec::new();
    };
    vec![Changed {
        op: "add",
        id: element.id.to_string(),
        alias: element.alias.as_ref().map(|alias| alias.to_string()),
    }]
}

fn added_statement(before: &Catalog, after: &Catalog) -> Vec<Changed> {
    let Some(stmt) = after
        .statements
        .iter()
        .find(|stmt| before.find_stmt(stmt.id).is_none())
    else {
        return Vec::new();
    };
    vec![Changed {
        op: "add",
        id: stmt.id.to_string(),
        alias: None,
    }]
}

