// Turning the command line's words into the model's vocabulary. This is where a
// `--body` becomes a parsed `Body`, a missing `--in` becomes the crate root, and a
// `--sig` that omits its keyword gets one. The model then decides legality.
//
// It deliberately does not run `plan`: building a change cannot fail for a model
// reason (only for a missing file or unparsable body), and separating the two keeps
// the refusal codes honest about which layer refused.

use super::error;
use super::report::{Changed, Error};
use crate::cli::request::{Add, Edit, Mv};
use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::element::Visibility;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;

mod body;

pub fn add(env: &super::Env<'_>, add: Add, stdin: &str) -> Result<Change, Error> {
    if add.kind == Kind::Stmt {
        return Err(error::unsupported("add", "add stmt (M7 owns statements)"));
    }
    if add.after.is_some() {
        return Err(error::unsupported("add", "--after"));
    }
    if add.doc.is_some() {
        return Err(error::unsupported("add", "--doc"));
    }
    if add.name.is_empty() {
        return Err(error::unsupported("add", "an empty name"));
    }

    let module = match add.kind {
        Kind::Module => add.name.clone(),
        _ => add.module.clone().unwrap_or_else(|| body::root_module(env)),
    };
    let signature = body::signature(&add);
    let body = match add.kind {
        Kind::Module => body::preamble(env, &add, stdin),
        _ => body::body(env, add.body.as_ref(), stdin, add.kind.needs_body()),
    };

    Ok(Change::AddElement {
        kind: add.kind,
        module,
        signature,
        visibility: add.visibility.unwrap_or(Visibility::Private),
        body: body?,
    })
}

pub fn edit(env: &super::Env<'_>, edit: Edit, stdin: &str) -> Result<Change, Error> {
    if edit.doc.is_some() {
        return Err(error::unsupported("edit", "--doc"));
    }
    let id = parse_id(&edit.id)?;
    let body = match edit.body.as_ref() {
        Some(source) => Some(body::body(env, Some(source), stdin, true)?),
        None => None,
    };
    if edit.rename.is_some()
        && (edit.signature.is_some() || edit.visibility.is_some() || body.is_some())
    {
        return Err(error::unsupported(
            "edit",
            "--rename with another edit (they are separate changes)",
        ));
    }
    if let Some(name) = &edit.rename {
        // A rename is a separate change because it rewrites references; combining
        // it with a head edit would make `plan` apply two intents as one.
        return Ok(Change::Rename {
            id,
            name: name.clone(),
        });
    }
    if edit.signature.is_none() && edit.visibility.is_none() && body.is_none() {
        return Err(Error {
            code: "usage.no_change",
            exit: super::Exit::Usage,
            message: "edit needs at least one of --rename, --sig, --vis or --body".to_string(),
        });
    }
    Ok(Change::EditElement {
        id,
        head: edit.signature,
        visibility: edit.visibility,
        body,
    })
}

pub fn remove(ids: &[String]) -> Result<Vec<Change>, Error> {
    if ids.is_empty() {
        return Err(Error {
            code: "usage.missing_argument",
            exit: super::Exit::Usage,
            message: "rm needs at least one id".to_string(),
        });
    }
    ids.iter().map(|id| Ok(Change::Remove { id: parse_id(id)? })).collect()
}

pub fn move_(mov: Mv) -> Result<Change, Error> {
    if mov.after.is_some() {
        return Err(error::unsupported("mv", "--after (statement reorder is M7)"));
    }
    let module = mov.module.ok_or_else(|| Error {
        code: "usage.missing_argument",
        exit: super::Exit::Usage,
        message: "mv needs a module".to_string(),
    })?;
    Ok(Change::MoveElement {
        id: parse_id(&mov.subject)?,
        module,
    })
}

/// What changed between two catalogs, in the envelope's words.
pub fn describe(change: &Change, before: &Catalog, after: &Catalog) -> Vec<Changed> {
    match change {
        Change::AddElement { kind, .. } => added_element(before, after, *kind),
        Change::AddStmt { .. } => added_statement(before, after),
        Change::EditElement { id, .. } => one("edit", *id, after),
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

fn parse_id(text: &str) -> Result<ElId, Error> {
    ElId::parse(text).ok_or_else(|| Error {
        code: "notfound.element",
        exit: super::Exit::NotFound,
        message: format!("{text:?} is not an element id"),
    })
}

