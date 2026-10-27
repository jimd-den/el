# 0020 — Read verbs are id-only, renderer-ordered, and read the catalog

Status: accepted · Date: 2026-10-26
Context: M7 adds the token-saving read surface: `el ls`, `el tree`, and `el show`.
An agent should learn every element id and one full body without opening a `.rs`
file. `COMMANDS.md` §4.3 is the contract; it leaves three decisions open — whether
a read verb refuses a drifted tree, whether `show` accepts an alias, and what the
`--json` payload beside the envelope looks like.

Decision:

1. **Read verbs answer from the catalog, not the disk.** They neither refuse a
   drifted tree nor repair an interrupted transaction: recovery needs the writer
   lock, and a read that took it would refuse to run during the very transaction it
   is reading about — or, worse, restore head's tree over a live writer's
   half-written projection. `run` holds the lock because it executes the files, so
   it may recover and refuses drift; `doctor --repair` recovers on request. `show
   --source` shows the committed projection, which is the model the journal owns.
   A read verb that silently read the hand-edited file would answer a question
   nobody asked.
2. **`show` is id-only.** It accepts `el-NNNN`, the form `edit` accepts. An alias
   such as `greet::fn/greet` is refused with `notfound.element` and exit 6; an id
   that is well formed but absent gets the same code. No alias lookup, no guessing.
3. **Ordering is the renderer's.** `ls` sorts by module path then id; `tree` walks
   `render::tree::Tree`, whose nodes are path-ordered and whose items are id-ordered;
   statements are `children_of` order. A read surface that invented its own order
   would disagree with the files the user sees.
4. **The envelope stays one envelope; verb data lives under `data`.** The existing
   `Report` gained an optional `payload`. `ls` emits an array, `tree` an object, and
   `show` an object. `statements` is always an array of ids; `--statements` adds
   `statement_tree` (id, text, children); `--source` and `--history` add their
   fields only when asked. Human and JSON are built from one `View`.
5. **`show`'s doc and sequence are derived, not stored.** `doc` is the module
   preamble for a module, and leading `///`/`//!` children for an item. `sequence`
   is the newest journal entry whose target/msg names the element, with the entry's
   kind token required for an `add`; it is `null` when the journal cannot prove one.
   Both are named as derived so a future stored field can replace them without
   changing the contract.

Alternatives rejected: reading the `.rs` files to answer `show` (the milestone
exists to avoid exactly that); alias lookup (ambiguous by design, and `edit`
already refuses it); refusing drift in `ls` (it answers from the catalog); a new
`--read` envelope (the existing `data` field is additive and already tested).

Consequences: `show sequence` is a best-effort journal answer, not a stored
revision; `doc` is a convention over statement text until M12's `--doc` lands.
Statement editing remains M7's other half and is not changed here. Revisit if the
journal starts recording a per-element revision, at which point `sequence` becomes
exact instead of inferred.
