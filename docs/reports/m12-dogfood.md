# M12 dogfood — `el` adopts, checks and edits its own source

Date: 2026-10-27 · Branch: `feat/m12-m13-agent-surface` · Base: `feat/agent-loop` (PR #11)

## What was run

`el` was built from this branch and run in this repository, which is a plain Cargo
crate with no `.el/` store. `el init` adopted the source; the store is untracked
and ignored by `.gitignore`.

```console
$ target/debug/el init
adopted 171 files as opaque modules
$ target/debug/el check
el: check ok (L1 check, 0 diagnostics, 99 ms)
$ target/debug/el log -n 3
000003  human  edit    check  edit el-0325
000002  human  import  skip   adopted 171 files as opaque modules
000001  human  init    model  created el-m12
```

The store is 3.5 MB: 171 blobs, the catalog, the journal and the two snapshots.

## The first edit of its own source

The file was `src/core/text/tsv.rs`, module `el-0325`. The change added one test
for the empty field, which the round-trip tests did not cover. The new file was
prepared out of tree, then applied by `el` at file granularity:

```console
$ target/debug/el edit el-0325 --body /tmp/tsv-new.rs --body-mode opaque
el-0325
el: rewrote 1 file
$ cargo test -q --lib an_empty_field_survives
test result: ok. 1 passed; 0 failed
```

The gate ran `cargo check` through the same parser as every other change, and the
journal recorded the edit as seq 3. This is file granularity, not element
granularity: an opaque module is one stored body, so `el add` or `el mv` into it
is refused with `model.opaque` and names the whole-file edit that works.

## Defects the dogfood found, and fixed

1. **`src/a/mod.rs` was unseen.** A grandchild in `src/a/b.rs` made the tree
   invent `src/a.rs`; the first adopted edit refused with “src/adapters.rs is
   missing”. The tree now prefers the catalog's own module path. Witness:
   `a_mod_rs_ancestor_keeps_its_own_path`.
2. **Every adopted file looked like a 170-line function.** `max_fn_lines` measured
   the opaque body and refused the edit with a wall of `model.fn_budget`. The rule
   now skips opaque elements; the file ceiling still applies. Witness:
   `an_opaque_file_is_not_a_function_body_budget`.
3. **The manifest was treated as a projection.** The first adopted edit added the
   rendered `Cargo.toml` to the new tree, overwrote the user's manifest, then the
   restore deleted it. The manifest is now never in a tree; `el new` writes it,
   and adoption and later changes leave it alone. The adoption test asserts it
   survives an edit and an undo.

## What this proves

- `el init` adopts a real crate byte for byte: the source, the manifest and the
  policy were unchanged after adoption.
- `el check` runs on a plain Cargo crate, and with a catalog it attributes
  diagnostics through the same span map the gate uses.
- `el edit` can change one adopted file, the gate can refuse it, and `el undo`
  restores the previous bytes.
- `make check` is green with `el check` on `el` itself.

## What this does not prove

- Element-level editing of adopted files. That needs `import --from-symbols`
  (ADR-0014), which is not built.
- That the tool is pleasant to drive. This is one session, not a benchmark; M13
  adds the harness for that question, not the answer.
- That the manifest is safe to edit through `el`. It is deliberately not owned.
