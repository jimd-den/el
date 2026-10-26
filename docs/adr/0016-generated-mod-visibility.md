# 0016 — A generated `mod` line is as visible as what it holds

Status: accepted · Date: 2026-10-25
Context: FR-15 says users never write `mod` declarations; the renderer derives them from file
paths. Paths carry no visibility, so the renderer has to decide one. The first version wrote
`mod name;` for every child, and `cargo check` refused it:

```text
error[E0603]: module `plan` is private
 --> src/main.rs:6:33
  |
6 |     println!("{}", crate::core::plan::apply(1));
  |                                ^^^^ private module
  |
 --> src/core.rs:3:1
  |
3 | mod plan;
```

The catalog said `pub fn apply`; the projection made it unnameable. That is the projection
contradicting the model, not a mistake by the user.

Decision: a node's visibility is the widest visibility of anything in its subtree, folded up
through every module above it, and the `mod` line that declares it uses that visibility
(`pub mod`, `pub(crate) mod`, `mod`). A module holding nothing `pub` stays private, so the
projection never exposes more than the catalog asked for.

| Catalog says | Projection writes |
|---|---|
| `pub fn apply` in `src/core/plan.rs` | `pub mod core;` in `src/main.rs`, `pub mod plan;` in `src/core.rs` |
| `fn helper` in `src/core/plan.rs` | `mod core;`, `mod plan;` |

Consequences:

- **Nothing is hidden and nothing is over-exposed.** Both halves matter: `pub` on every
  generated line would work and would turn private structure into the crate's semver surface.
- **The rule is derived, so it cannot drift.** No new field, no policy in the catalog, no
  user-visible flag. Changing a `pub` on one item re-derives the whole chain above it.
- **A `pub` item in a binary crate gains a `pub mod`.** Pointless to the compiler, and kept
  anyway: one rule for both crate kinds is cheaper to reason about than two, and the
  alternative is the renderer guessing the crate kind from the root file's name.

Alternatives rejected: `pub(crate) mod` everywhere (hides `pub` items from outside a library,
so it breaks the one case that matters most); `pub mod` everywhere (publishes internal
structure as API); a `Visibility` field on `Kind::Module` elements (the intermediate modules
have no element at all — `src/core.rs` may exist only because `src/core/plan.rs` does); reading
the usages to decide the minimum (that is name resolution, and `el` does not resolve names).

Revisit if: the catalog gains explicit module elements with their own visibility, at which
point an explicit value should win over the derived one.

Refs: FR-14, FR-15; ADR-0009 (the projection is owned), ADR-0002 (the catalog is authoritative)
