# 0002 — The catalog is the source of truth; the files are a projection

Status: accepted · Date: 2026-10-24
Context: the tool must address *elements*, not files, and must be able to rebuild a file from
its model. Two designs are possible. (a) The `.rs` files are the truth and the tool parses
them. (b) The model is the truth and the files are rendered from it.

Decision: (b). `.el/catalog` (rebuilt from the journal) holds modules, elements, statements,
dependencies and policy. `src/**` and `Cargo.toml` are a **projection**: derived, disposable,
regenerable, and owned by `el`. Every generated file's first line says so.

Alternatives rejected:
- (a) requires a Rust parser, which is either a giant dependency (ADR-0001) or a project of
  its own. Without parsing, "edit the element" degrades to "guess at the text", which is the
  failure mode this tool exists to remove.
- A hybrid ("parse what we can") was rejected: a model that is sometimes authoritative and
  sometimes not is a model nobody can reason about, and the gate's span map depends on knowing
  the model is right.

Consequences: hand edits to managed files are drift, detected and refused (ADR-0009).
`cargo fmt` on a managed module is undone. Users who want a normal file can adopt it as opaque
and then `el` stops modelling it — the escape hatch exists, and it is explicit.
Payoff: rendering is a pure function (so determinism, the span map, and re-rendering after a
failed gate are all trivial), and history is exact because it snapshots what we rendered.

Revisit if: `el import` (ADR-0014) reaches parity, at which point "files as truth" becomes a
supported mode rather than a degraded one.
