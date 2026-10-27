# 0028 — Opaque adoption, and the manifest is not a projection
Status: accepted · Date: 2026-10-27

Context: `el init` must bring an existing Cargo crate into the model without
rewriting it. ADR-0009 already says an adopted file is opaque and rendered byte
for byte, but three questions were open: what one adopted file is in the model,
who owns `Cargo.toml`, and what an element-level edit of an opaque file does.

Decision:

- One `src/**/*.rs` file is one `mod` element whose body is the whole file,
  stored as one opaque statement. The renderer returns that statement verbatim
  and records one span for the file, so diagnostics fall back to file/line.
- `Cargo.toml` is never in a tree. `el new` writes the rendered manifest; `init`
  and every later change leave the existing manifest alone. A tree that cannot
  name the manifest cannot restore over it or delete it.
- An item cannot be added or moved into an opaque module: it would exist in the
  catalog and not in the file. The refusal is `model.opaque` and names the
  whole-file edit that works: `el edit <module> --body <file> --body-mode opaque`.
  Editing the module's body is allowed and is the file-level edit.
- A module's file path wins over the `src/a.rs` default: if the catalog already
  has `src/a/mod.rs`, a grandchild does not invent `src/a.rs`.

Alternatives rejected: parse the file on adoption (ADR-0013 refuses to guess);
own the manifest (a rewrite is not adoption); render items into an opaque file
(would break the byte-for-byte promise); one element per file with no module
identity (nothing to address).

Consequences: adoption is one store write with no gate, because the tree is
already on disk and verified byte for byte. Element-level editing of an adopted
file waits for `import --from-symbols` (ADR-0014). The file ceiling still applies
to an opaque module; `max_fn_lines` does not, because there is no modelled body.

Revisit if: `import` can split an adopted file, at which point opaque becomes a
transitional state rather than the supported end state.
