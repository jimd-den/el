// The compiler gate: how a prepared change is judged before it may commit.
// One question, answered in order: which tier, what does the tool say, what
// is new versus already broken. A tier that cannot run is an error, never a
// pass; a change that adds an error never commits, whatever the tier.

use crate::app::error::Error;
use crate::app::{Env, Exit};
use crate::core::gate::{delta, verdict, Delta, Diagnostic, Tier};
use crate::core::history::object::Digest;
use crate::core::history::tree::Tree;
use crate::core::model::catalog::Catalog;
use crate::core::render::Projection;

mod baseline;
mod run;

pub(crate) use baseline::write_baseline;

/// What the gate needs from the prepared change. Lifetimes tie every borrowed
/// piece to the transaction, never to a shorter scope.
pub struct Input<'e, 'p> {
    pub env: &'e Env<'e>,
    pub tier: Tier,
    pub catalog: &'p Catalog,
    pub old_catalog: &'p Catalog,
    pub projection: &'p Projection,
    pub old_tree: &'p Tree,
    pub new_tree: &'p Tree,
    pub old_digest: Digest,
}

/// The judged change: its tier, its delta against the baseline, and the full
/// current diagnostic set (what a later baseline write records).
pub struct Evaluation {
    pub delta: Delta,
    pub current: Vec<Diagnostic>,
}

/// The tier a change runs under. Fast never runs a tool; strict always runs
/// the strict tier; otherwise the policy names it. A policy tier el does not
/// build is refused, never silently downgraded.
pub fn select(policy_gate: &str, fast: bool, strict: bool) -> Result<Tier, Error> {
    if fast {
        return Ok(Tier::Model);
    }
    if strict {
        return Ok(Tier::Strict);
    }
    match policy_gate {
        "model" => Ok(Tier::Model),
        "check" => Ok(Tier::Check),
        "strict" => Ok(Tier::Strict),
        "semantic" => Err(Error {
            code: "usage.unsupported",
            exit: Exit::Usage,
            message: "el.policy gate=semantic is not supported yet (M10)".to_string(),
        }),
        _ => Err(Error {
            code: "env.bad_policy",
            exit: Exit::Env,
            message: format!("el.policy gate={policy_gate} is not a tier"),
        }),
    }
}

/// Runs the tier's tools and diffs the result against the baseline. The
/// model tier never touches a tool or a baseline: it has nothing to judge.
pub fn evaluate(input: Input) -> Result<Evaluation, Error> {
    if input.tier == Tier::Model {
        return Ok(Evaluation {
            delta: Delta { new: vec![], fixed: vec![], existing: vec![] },
            current: vec![],
        });
    }
    let current = run::run_tier(input.env, input.tier, input.catalog, input.projection)?;
    let baseline = baseline::load_or_compute_baseline(&input)?;
    let delta = delta(&baseline, &current);
    Ok(Evaluation { delta, current })
}

/// Whether the judged change may commit: the model tier always passes, the
/// check tier blocks only on new errors, the strict tier blocks on any new
/// diagnostic. Re-exported so the transaction reads one rule in one place.
pub fn may_commit(tier: Tier, delta: &Delta) -> bool {
    verdict(tier, delta)
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
