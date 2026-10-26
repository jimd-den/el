# 0023 — A block balances over its subtree, and a module reference names one file

Status: accepted · Date: 2026-10-26
Context: building a real program against M7/M8 found three breaks, all in the
same shape — one thing with two identities. A multi-line block was refused as
unbalanced; a bare module name became a phantom tree node no file renders;
and `add mod` with a bare name collided with itself.

Decision:

1. **L0 judges balance over a statement subtree, not per statement.** The
   grammar stores a block as a parent statement plus children, and the
   renderer closes it, so a parent alone looks unbalanced. Validation now
   simulates what the renderer writes — the statement, its children in source
   order, and the `}` the renderer emits after a block opener's children —
   and reports at the subtree root, so one bad subtree is one violation.
   A childless opener still refuses: it gets no closer, exactly as the
   grammar refuses a block that never gains a body. An empty block is
   therefore written single-line (`if x {}`), or it gains a child.
2. **A module reference is normalized at the command-line boundary.**
   `catalog`, `src/catalog.rs`, `core/plan` and `core::plan` all name
   `src/catalog.rs` / `src/core/plan.rs`. The catalog stores file paths and
   the tree derives nodes from them, so a bare name stored verbatim was a
   second identity for the same module: the tree showed it as a node no file
   renders, and seeding created a module element whose alias collided with
   the one the user had just added. Normalization covers `add mod`,
   `--in` and `mv --module`; `ls --module` already matched this way and is
   unchanged.

Alternatives rejected: keeping the per-statement check (the bug — it can only
pass flat bodies and single-line blocks, which is most of nothing real);
refusing bare module names instead of normalizing them (fails closed, but
agents write bare names constantly and the alias system already equates the
spellings, so refusal would be the tool disagreeing with itself);
normalizing in the renderer (too late — the tree nodes and the seeded module
have already diverged by then).

Consequences: multi-line `if`/`for`/`loop`/`match` bodies pass L0 as split
statements and stay statement-editable; a two-module program built only with
el verbs compiles and runs. Unchanged: opaque bodies still skip the balance
check; `else` after a closed block is still refused by the grammar; a truly
unclosed delimiter is still refused, named, with nothing written.
