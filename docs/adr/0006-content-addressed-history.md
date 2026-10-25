# 0006 — Content-addressed history instead of shelling out to git

Status: accepted · Date: 2026-10-24
Context: `undo` must work on a bare machine, must be exact, and must not require a repository
the user did not ask for. `git` is present on most developer machines but not all, and it is
not present in the minimal container this tool targets.

Decision: a small append-only store: SHA-256 blobs, tree objects listing `path → digest`,
one file per journal entry, and `head` as a single integer. Git is neither required nor invoked.

Alternatives rejected:
- **Shell out to `git`.** Breaks the promise on machines without git (ADR-0001's whole point),
  pollutes the user's real history with one commit per statement edit, and needs a commit
  *before* the write rather than after.
- **A library like `git2`.** Pulls `libgit2-sys` and a build dependency — the opposite of P3.
- **Plain full-tree copies.** Simple but every snapshot duplicates every file; diffs and
  regression checks then need a separate index. Content addressing costs ~110 lines and buys
  dedupe plus integrity.

Consequences: we implement and test hashing, trees and encoding (≈370 lines across four
files), and we own crash safety. We do **not** get packing, remotes, merge or `git log`.
`.el/` is gitignored; the projection remains a normal working tree so users may commit it
themselves.

Revisit if: users want shared or remote history — that is an export adapter, not a rewrite,
because the objects are already content-addressed.
