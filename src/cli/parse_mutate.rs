// The two mutating verbs that take a rich flag set: `add` and `edit`.
//
// They live apart from `parse` because they are where "a flag the parser ignores"
// would be cheapest to introduce: every value they accept is converted here, so a
// wrong word becomes a `Usage` before staging, not a field nobody reads. The
// dispatch table in `parse` answers which verb; this answers what its arguments
// mean.

use super::args::{Args, Usage};
use super::request::{Add, Edit, Source};
use crate::core::model::element::Visibility;
use crate::core::model::kind::Kind;
use crate::core::plan::policy::Split;

pub(super) fn add(args: &Args) -> Result<Add, Usage> {
    let kind = args
        .words
        .get(1)
        .and_then(|word| Kind::parse(word))
        .ok_or_else(|| {
            Usage::new(
                "usage.unknown_kind",
                format!(
                    "add needs a kind, one of: {}",
                    Kind::all().iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" ")
                ),
            )
        })?;
    let name = args
        .words
        .get(2)
        .cloned()
        .ok_or_else(|| Usage::new("usage.missing_argument", format!("add {kind} needs a name")))?;

    Ok(Add {
        kind,
        name,
        module: args.value("--in")?,
        visibility: visibility(args.value("--vis")?)?,
        signature: args.value("--sig")?,
        body: source(args.value("--body")?),
        doc: source(args.value("--doc")?),
        preamble: source(args.value("--preamble")?),
        after: args.value("--after")?,
        before: args.value("--before")?,
        at: at(args.value("--at")?)?,
    })
}

pub(super) fn edit(args: &Args) -> Result<Edit, Usage> {
    Ok(Edit {
        id: args.required("edit")?,
        rename: args.value("--rename")?,
        signature: args.value("--sig")?,
        body: source(args.value("--body")?),
        doc: source(args.value("--doc")?),
        visibility: visibility(args.value("--vis")?)?,
        mode: body_mode(args.value("--body-mode")?)?,
    })
}

/// `--body-mode` is the command-line override of the policy's split setting.
/// `None` means "the project decides", which is not the same as `Split::Always`.
fn body_mode(value: Option<String>) -> Result<Option<Split>, Usage> {
    match value.as_deref() {
        None => Ok(None),
        Some("split") => Ok(Some(Split::Always)),
        Some("opaque") => Ok(Some(Split::Never)),
        Some(other) => Err(Usage::new(
            "usage.bad_value",
            format!("--body-mode takes split or opaque, not {other:?}"),
        )),
    }
}

fn visibility(value: Option<String>) -> Result<Option<Visibility>, Usage> {
    match value {
        None => Ok(None),
        Some(token) => Visibility::parse(&token).map(Some).ok_or_else(|| {
            Usage::new(
                "usage.bad_visibility",
                format!("--vis takes pub, crate or priv, not {token:?}"),
            )
        }),
    }
}

fn source(value: Option<String>) -> Option<Source> {
    value.map(|text| Source::parse(&text))
}

fn at(value: Option<String>) -> Result<Option<usize>, Usage> {
    match value {
        None => Ok(None),
        Some(text) => text.parse().map(Some).map_err(|_| {
            Usage::new("usage.bad_value", format!("--at takes a number, not {text:?}"))
        }),
    }
}

#[cfg(test)]
#[path = "parse_edit_tests.rs"]
mod parse_edit_tests;
