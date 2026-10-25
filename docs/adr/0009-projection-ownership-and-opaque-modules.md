# 0009 — The projection is owned; unmodelled files are opaque

Status: accepted · Date: 2026-10-24
Context: ADR-0002 makes the catalog authoritative, so a hand edit to a managed file makes the
model and the disk disagree. Something must give, and there are three things a user legitimately
wants to do: edit a managed file by hand, run `cargo fmt`, or bring an existing file into the
project.

Decision: three explicit states, no silent merging.

| State | Trigger | Behaviour |
|---|---|---|
| **Owned** | created by `el` | `el` renders it; content differing by more than whitespace is an error (exit 4) naming the file |
| **Drift (whitespace)** | `cargo fmt`, an editor's trim | re-rendered, with one stderr line saying so |
| **Opaque** | `el init`, `doctor --adopt`, `import --from-symbols` | stored and rendered byte-for-byte; element-level edits refused with an explanation; everything else (gate, history, build, run) still works |

The ownership marker on line 1 of every owned file states the contract in the file itself:

```rust
// @el-projection — generated from the catalog; change it with `el edit`, not here.
```

Alternatives rejected: parse the file to reconcile drift (needs a parser); silently absorb
hand edits (the model stops being true, and every later edit is built on a lie); refuse
`cargo fmt` (hostile to a tool everyone runs).

Consequences: opaque modules are a first-class, honest degrade — the tool says "I do not model
this file" instead of guessing. Their diagnostics fall back to line-sensitive fingerprints
(ADR-0005). A user who wants element-level editing on an adopted file gets it at M12 via
`import --from-symbols` (ADR-0014) or by re-creating the element and deleting the file.

Revisit if: a real parser becomes affordable, at which point opaque becomes a legacy state
rather than a supported one.
