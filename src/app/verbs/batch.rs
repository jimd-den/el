// `el batch`: many adds as one transaction. The input is a JSON array on
// stdin, one object per `add`; every item becomes a `Change` through
// `change::add`, and all of them ride one `Mutation`, so the plan, the
// gate and the commit each happen once — a failure anywhere applies
// nothing. Statements are refused: they need ids that do not exist until
// planning.

use super::super::change;
use super::super::error::{self, Error};
use super::super::load;
use super::super::transaction::Mutation;
use super::super::{Env, Report};
use super::mutate;
use crate::cli::request::{Add, Globals, Source};
use crate::core::history::entry::Op;
use crate::core::model::element::Visibility;
use crate::core::model::kind::Kind;
use crate::core::text::json::{self, Json};

/// Parse the batch input into `add` requests. Pure: no filesystem, no
/// catalog, so a malformed item is a usage error naming its index.
pub fn parse_batch(stdin: &str) -> Result<Vec<Add>, Error> {
    let Some(Json::Array(items)) = json::parse(stdin) else {
        return Err(usage("batch input must be a JSON array of items"));
    };
    if items.is_empty() {
        return Err(usage("batch needs at least one item"));
    }
    let mut adds = Vec::new();
    for (index, item) in items.iter().enumerate() {
        adds.push(parse_item(item, index)?);
    }
    Ok(adds)
}

fn parse_item(item: &Json, index: usize) -> Result<Add, Error> {
    let field = |name: &str| item.get(name).and_then(Json::as_str).map(str::to_string);
    let kind = field("kind").ok_or_else(|| item_error(index, "needs a kind"))?;
    let kind = Kind::parse(&kind)
        .ok_or_else(|| item_error(index, &format!("unknown kind {kind:?}")))?;
    if kind == Kind::Stmt {
        return Err(error::unsupported("batch", "stmt items"));
    }
    let name = field("name").ok_or_else(|| item_error(index, "needs a name"))?;
    if name.is_empty() {
        return Err(item_error(index, "needs a non-empty name"));
    }
    let visibility = match field("vis") {
        None => None,
        Some(token) => Some(
            Visibility::parse(&token)
                .ok_or_else(|| item_error(index, &format!("unknown visibility {token:?}")))?,
        ),
    };
    let body = field("body").map(Source::Literal);
    let (body, preamble) = if kind == Kind::Module {
        (None, body)
    } else {
        (body, None)
    };
    Ok(Add {
        kind,
        name,
        module: field("in"),
        visibility,
        signature: field("sig"),
        body,
        doc: None,
        preamble,
        after: None,
        before: None,
        at: None,
    })
}

fn item_error(index: usize, problem: &str) -> Error {
    usage(&format!("batch item {index}: {problem}"))
}

fn usage(message: &str) -> Error {
    Error {
        code: "usage.bad_batch",
        exit: super::super::Exit::Usage,
        message: message.to_string(),
    }
}

pub fn batch(env: &Env<'_>, globals: &Globals, stdin: &str) -> Report {
    let split = match load::policy(env) {
        Ok(policy) => policy.split,
        Err(error) => return Report::fail("batch", error),
    };
    let adds = match parse_batch(stdin) {
        Ok(adds) => adds,
        Err(error) => return Report::fail("batch", error),
    };
    let mut changes = Vec::new();
    for add in &adds {
        match change::add(env, add.clone(), "", split) {
            Ok(change) => changes.push(change),
            Err(error) => return Report::fail("batch", error),
        }
    }
    let count = changes.len();
    mutate(
        env,
        globals,
        Mutation {
            cmd: "batch",
            op: Op::Add,
            target: format!("batch {count}"),
            message: format!("batch {count} adds"),
            changes,
            origins: Vec::new(),
            dry_run: globals.dry_run,
            strict: globals.strict,
            fast: globals.fast,
        },
    )
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod batch_tests;
