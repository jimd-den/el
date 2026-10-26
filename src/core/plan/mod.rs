// Planning: a change either produces a new catalog and a projection, or a set of
// refusals that names what would have to change. Nothing here touches a file, a
// clock or a process, and nothing here mutates the catalog it is given — the
// caller keeps its state until the gate has spoken (FR-18, FR-21).
//
// The order inside `plan` is the order of the costs it pays: the request is
// checked first (free, and the message names ids the user still recognises), then
// the change is applied to a copy, then the copy is rendered, then L0 reads the
// result. A refusal at any step returns before the next one runs, so a refused
// change has computed nothing it did not have to.
//
// The rules live in `validate` (state) and `request` (the ask); the mutations in
// `apply_element` and `apply_stmt`; the vocabulary is `model::change`.

mod apply_element;
mod apply_stmt;
pub mod policy;
pub mod refs;
pub mod rendered;
pub mod request;
pub mod validate;
pub mod violation;

pub use violation::{Code, Violation, Violations};

use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::id::ElId;
use crate::core::render::{self, Projection};

/// What was asked for, with the project's opinion and the project's name.
///
/// A parameter object rather than three arguments, because a policy and a crate
/// name travel together through every caller and neither belongs to the change.
pub struct Request<'a> {
    pub change: &'a Change,
    pub policy: &'a policy::Policy,
    pub crate_name: &'a str,
}

/// A text that a rename changed, kept whole so history can replay it and a test
/// can prove what was *not* touched (FR-2, R2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub site: ElId,
    pub before: String,
    pub after: String,
}

/// The result of a change that was allowed: the new catalog, the text it renders
/// to, and any reference rewrite the change implied.
#[derive(Debug)]
pub struct Outcome {
    pub catalog: Catalog,
    pub projection: Projection,
    pub rewrites: Vec<Rewrite>,
}

/// Apply `asked` to `catalog`, or explain why it cannot be.
///
/// Contract:
///   - Pure: `catalog` is only read, never mutated, so a caller that has the
///     catalog in hand cannot be left with a half-applied change.
///   - Every refusal is a non-empty `Violations`; a change either happens whole
///     or not at all.
pub fn plan(catalog: &Catalog, asked: &Request<'_>) -> Result<Outcome, Violations> {
    let refused = request::request(asked.change, catalog, asked.policy);
    if let Some(violations) = Violations::new(refused) {
        return Err(violations);
    }
    if let Change::AddDep { name } = asked.change {
        return Err(Violations::one(unrepresentable(name)));
    }

    let mut next = catalog.clone();
    let applied = apply_element::apply(&mut next, asked.change)
        .and_then(|()| apply_stmt::apply(&mut next, asked.change));
    if let Err(violation) = applied {
        return Err(Violations::one(violation));
    }

    let rewrites = match asked.change {
        Change::Rename { id, .. } => rename(&mut next, *id, catalog),
        _ => Vec::new(),
    };

    let projection = render::render(&next, asked.crate_name);
    match Violations::new(validate::state(&next, &projection, asked.policy)) {
        Some(violations) => Err(violations),
        None => Ok(Outcome {
            catalog: next,
            projection,
            rewrites,
        }),
    }
}

/// The old name can only be read from the catalog *before* the rename, which is
/// why `plan` still holds it after working on a copy.
fn rename(next: &mut Catalog, id: ElId, before: &Catalog) -> Vec<Rewrite> {
    let Some(old) = before.find(id).and_then(|element| element.name.clone()) else {
        return Vec::new();
    };
    refs::rename_references(next, id, &old)
}

/// FR-16 says a manifest declares no dependencies *unless the catalog declares
/// them*, and the catalog has no dependency record yet — that arrives with the
/// store. Until then an allowed dependency is refused rather than dropped, because
/// accepting it and writing a manifest without it is the silent deletion this
/// codebase exists to avoid.
fn unrepresentable(name: &str) -> Violation {
    Violation::new(
        Code::DepDenied,
        format!(
            "`{name}` is allowed by el.policy, but this catalog cannot record a dependency yet; \
             edit Cargo.toml by hand until the store can"
        ),
    )
}

#[cfg(test)]
pub(crate) mod fixture;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
