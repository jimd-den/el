# 0017 — A refusal is a value, and `plan` works on a copy

Status: accepted · Date: 2026-10-25
Context: FR-18 says L0 runs on every mutation, costs ≤ 20 ms, and writes nothing on failure;
FR-21 says a rejected change leaves the projection and the head untouched. Both are promises
about what *did not happen*, and a promise about a non-event is only worth what its structure
is worth. A `plan(catalog, change) -> Projection | Violations` shape with two out-parameters
would put "did it change anything?" back into the caller's hands, and every caller would have
to get it right.

Decision, in three parts.

**1. `plan` reads its input and returns an outcome.** Signature: `plan(&Catalog, &Request) ->
Result<Outcome, Violations>`, where `Outcome` carries a *new* catalog, the projection and any
reference rewrite the change implied. `&Catalog` makes mutation unrepresentable rather than
forbidden, so the "catalog is byte-identical after a refusal" property follows from the type,
not from discipline. Every step works on a clone, and the clone is what L0 reads and what the
caller keeps on success.

**2. A refusal is a non-empty set, never an empty one.** `Violations` cannot be constructed
from an empty `Vec`. `Ok` and `Err` therefore mean "every rule passed" and "at least one rule
did not", with no fourth state — the shape a caller cannot misread is the shape a caller
cannot get wrong. Each violation carries the rule's code (one `Code` per rule in `GATE.md` §5,
spelled by an exhaustive match so a new rule cannot ship without a code), the element it is
about, the measured size when there is one, and the smallest action that would satisfy the
rule.

**3. Rules are split by what can answer them.** `request` holds the three rules about the
change itself (cross-body move, denied dependency, removal leaving a reference) and runs before
anything is applied; `validate` holds the five rules about the catalog; `rendered` holds the
three that need the text as it would be written (ceiling, function budget, preamble). The
order is the order of cost, and the first refusal is the cheapest one, which is why a request
refusal names ids the user still recognises.

Consequences:

- **A refusal costs one clone and one render**, and never a write. `--dry-run` becomes a
  byproduct of the normal path rather than a second, less-tested path.
- **Idempotence is free.** Planning the same change against the same catalog gives the same
  outcome, because nothing outside the arguments is read: no clock, no environment, no
  filesystem.
- **The rename rewrite is part of the plan, not an afterthought.** `Outcome::rewrites` records
  each text that changed, before and after, so history (M8) can replay it and a test can prove
  what was *not* touched. `plan::refs` refuses wholesale: one unscannable body means no rewrite
  at all, because a rename that rewrote most references is worse than one that rewrote none.
- **A dependency change is refused twice for two different reasons.** When `el.policy` denies
  it, `model.dep_denied` names the policy. When the policy *allows* it, the same code names the
  real obstacle: the catalog has no dependency record yet, so accepting the change and writing
  a manifest without it would be silent deletion. This is accepted debt, not a design.

Accepted debt: `Change::AddDep` becomes a real change when the store lands (M5) and the
catalog gains a dependency record — FR-16's "unless the catalog declares one" has no catalogue
side yet. Until then `plan` refuses rather than drops, and the remedy says which of the two
reasons applies.

Alternatives rejected: mutating the catalog and rolling back on refusal (two code paths, and
the rollback is the one that is never tested); returning `(catalog, Vec<Violation>)` and letting
the caller decide (an empty vector means success, and one `if` in the wrong place writes a
rejected change); a `bool` on `Outcome` for "was anything rewritten" (the rewrites are already
the answer).

Revisit if: the clone becomes the wrong cost — a catalog large enough that copying it twice per
change is measurable against FR-18's 20 ms. The fix then is a copy-on-write catalog, not a
mutable `plan`.

Refs: FR-18, FR-21, FR-26, FR-73; GATE.md §5; ADR-0002 (the catalog is authoritative)
