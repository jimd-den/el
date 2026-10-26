# 0022 — History is head moves, an attic, and per-sequence catalog snapshots

Status: accepted · Date: 2026-10-26
Context: M8 adds undo, redo, revert and diff. The store already had trees and a
journal, but no catalog for a past sequence, so an element-level diff or a restored
catalog could not be answered. The milestone also needed a rule for what happens to
the future when a new mutation follows an undo.

Decision:

1. **Undo and redo are head moves plus a full materialize.** `undo n` moves head
   back n, restores that entry's tree (including deletions), and installs that
   sequence's catalog. `redo` does the same forward. There is no reverse diff, so a
   half-applied undo is not representable (FR-29).
2. **Every committed sequence gets a catalog snapshot.** `.el/catalogs/<seq>` holds
   the catalog text the transaction committed; the head's `.el/catalog` is the
   cache. A store created before M8 has no old snapshots; undo to such a sequence is
   a named `env.corrupt` rather than a guessed catalog. `doctor --rebuild` writes
   the head snapshot back byte for byte (FR-34).
3. **A mutation after undo moves the future to `.el/attic/`.** The move happens
   after the plan succeeds and before the new entry is staged. Attic entries keep
   their bytes and are readable with `log --attic`; `redo` is gone because the next
   journal sequence is now the new entry (FR-30). A dry-run uses a sequence past
   both journal and attic, so it never overwrites a future.
4. **Revert appends.** It records a new entry whose `to` tree equals the target
   sequence's tree, and installs the target's catalog; the journal is never
   rewritten (FR-31).
5. **Diff compares from the older sequence to the newer.** No arguments means
   parent → head; one argument means that sequence → head; two mean `a → b`. When
   both have catalog snapshots, the payload carries element/statement ids and line
   changes; otherwise it falls back to the file line diff. The human view names the
   statement id, never a line range (FR-32).
6. **Doctor is explicit recovery.** `--repair` performs the same inflight rollback
   every command does and reports it; `--rebuild` restores the catalog cache from
   the head snapshot.

Alternatives rejected: reverse diffs for undo (cannot be exact for rendered text);
a branch/DAG history (the UI cost v1 cannot pay); rewriting history for revert
(makes the journal unable to explain the tree); deleting the future (attic costs
almost nothing and preserves the audit trail).

Consequences: old sequences from a pre-M8 store have no catalog snapshot; the
command refuses rather than guessing, and a future migration can rebuild them from
a replayed journal. `log` still shows future entries in the journal until a new
mutation moves them, so a reader must look at `head`, not the newest journal line.
`diff` without snapshots is file-level, not element-level. Revisit if the journal
grows a per-entry change record, at which point snapshots become reproducible
rather than stored.
