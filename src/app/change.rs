// Turning the command line's words into the model's vocabulary. This is where a
// `--body` becomes a parsed `Body`, a missing `--in` becomes the crate root, and a
// `--sig` that omits its keyword gets one. The model then decides legality.
//
// It deliberately does not run `plan`: building a change cannot fail for a model
// reason (only for a missing file or unparsable body), and separating the two keeps
// the refusal codes honest about which layer refused.

use super::error;
use super::report::Error;
use crate::cli::request::{Add, Edit, Mv};
use crate::core::model::change::{Anchor, Change};
use crate::core::model::element::Visibility;
use crate::core::model::id::ElId;
use crate::core::model::kind::Kind;
use crate::core::plan::policy::Split;

mod body;
mod describe;

pub use describe::describe;

pub fn add(env: &super::Env<'_>, add: Add, stdin: &str, split: Split) -> Result<Change, Error> {
    if add.kind == Kind::Stmt {
        return add_statement(env, &add, stdin);
    }
    if add.after.is_some() || add.before.is_some() || add.at.is_some() {
        return Err(error::unsupported("add", "an anchor on a non-statement"));
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
    let (body, mode) = match add.kind {
        Kind::Module => (body::preamble(env, &add, stdin)?, crate::core::model::element::BodyMode::Split),
        _ => body::element_body(env, add.body.as_ref(), stdin, add.kind.needs_body(), split)?,
    };

    Ok(Change::AddElement {
        kind: add.kind,
        module,
        signature,
        visibility: add.visibility.unwrap_or(Visibility::Private),
        body,
        mode,
    })
}

pub fn edit(env: &super::Env<'_>, edit: Edit, stdin: &str, split: Split) -> Result<Change, Error> {
    if edit.doc.is_some() {
        return Err(error::unsupported("edit", "--doc"));
    }
    let id = parse_id(&edit.id)?;
    let parsed = match edit.body.as_ref() {
        Some(source) => Some(body::element_body(env, Some(source), stdin, true, split)?),
        None => None,
    };
    let body = parsed.as_ref().map(|(body, _)| body.clone());
    let mode = parsed.map(|(_, mode)| mode);
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
    if edit.signature.is_none() && edit.visibility.is_none() {
        return Ok(Change::EditBody {
            id,
            body: body.expect("body is present"),
            mode,
        });
    }
    Ok(Change::EditElement {
        id,
        head: edit.signature,
        visibility: edit.visibility,
        body,
        mode,
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
    if let Some(after) = &mov.after {
        if mov.module.is_some() {
            return Err(error::unsupported("mv", "--after with a module"));
        }
        return Ok(Change::ReorderStmt {
            id: parse_id(&mov.subject)?,
            after: parse_id(after)?,
        });
    }
    let module = mov.module.ok_or_else(|| Error {
        code: "usage.missing_argument",
        exit: super::Exit::Usage,
        message: "mv needs a module or --after".to_string(),
    })?;
    Ok(Change::MoveElement {
        id: parse_id(&mov.subject)?,
        module,
    })
}

fn add_statement(env: &super::Env<'_>, add: &Add, stdin: &str) -> Result<Change, Error> {
    let owner = parse_id(&add.name)?;
    let source = add.body.as_ref().ok_or_else(|| Error {
        code: "usage.missing_argument",
        exit: super::Exit::Usage,
        message: "add stmt needs --body".to_string(),
    })?;
    let body = body::body(env, Some(source), stdin, true)?;
    let anchor = match (&add.after, &add.before, add.at) {
        (Some(after), None, None) => Anchor::After(parse_id(after)?),
        (None, Some(before), None) => Anchor::Before(parse_id(before)?),
        (None, None, Some(at)) => Anchor::At(at),
        (None, None, None) => Anchor::End,
        _ => {
            return Err(error::unsupported(
                "add stmt",
                "more than one of --after, --before, --at",
            ));
        }
    };
    Ok(Change::AddStmt { owner, body, anchor })
}

pub fn parse_id(text: &str) -> Result<ElId, Error> {
    ElId::parse(text).ok_or_else(|| Error {
        code: "notfound.element",
        exit: super::Exit::NotFound,
        message: format!("{text:?} is not an element id"),
    })
}

