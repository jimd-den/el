// Turning an `add stmt` command into a model change: the subject is
// the owning function, the body comes from a file or stdin, and the
// anchor is exactly one of --after, --before, --at, or the end.

use super::super::error;
use super::super::report::Error;
use super::{body, parse_id};
use crate::cli::request::Add;
use crate::core::model::change::{Anchor, Change};

pub(super) fn add_statement(env: &super::super::Env<'_>, add: &Add, stdin: &str) -> Result<Change, Error> {
    // The stmt subject is the owning function, not a name for the
    // statement — so the generic "resolve a name with find" hint
    // would send the agent hunting for an element that cannot
    // exist. Name the rule instead.
    let owner = parse_id(&add.name).map_err(|error| {
        if error.code == "notfound.element" {
            Error {
                message: format!(
                    "{:?} is not an element id; add stmt takes the owning function's id, not a name for the statement",
                    add.name
                ),
                ..error
            }
        } else {
            error
        }
    })?;
    let source = add.body.as_ref().ok_or_else(|| Error {
        code: "usage.missing_argument",
        exit: super::super::Exit::Usage,
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
