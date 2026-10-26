# 0018 — The store's four formats, and the component that is deliberately missing

Status: accepted · Date: 2026-10-25
Context: M5 has to make a snapshot worth restoring from. `HISTORY.md` §2–§4 already fixes the
layout and the record formats, so the question here is not *what* to write but *where the
guarantees live*: which module may declare a format, which may touch a disk, and what a reader
is entitled to assume about bytes it just read. Four schemas and one port enter the codebase in
this milestone, and a schema that is not written down is a schema that will be guessed at by the
next reader.

Decision, in five parts.

**1. A blob has no header. Its path is its digest.** `objects/<ab>/<cdef…>` holds the raw bytes
of one projection file. Verification is therefore *re-hashing*, not parsing: a reader that gets
the wrong bytes learns so without a decoder, and a file on disk is byte-for-byte the file the
compiler will see. A header would have bought framing we do not need and cost dedupe we do:
identical source in two crates is one object, and `doctor --prune` never has to unwrap a blob to
know what it holds.

**2. A tree is sorted `path ⇥ digest` lines, and duplicate paths are refused.** `Tree::new` sorts
and refuses a repeated path; `decode` routes through `new`, so a hand-edited tree cannot smuggle
in two answers for one file. Sorting is not tidiness — it is what makes one projection have one
name, the same argument as `render`'s deterministic order. A mode bit, a symlink or a directory
is out of scope: a projection is generated source plus a manifest, and the catalog has no field
that could produce one.

**3. An entry is ten fields, `el1` first.** The tag leads so a reader that was never taught `el2`
says exactly that instead of shifting every field by one. Free text is sanitized on write (`\t`,
`\n`, `\r` → space) and a message is truncated at 200 characters: a record that can contain a
newline is a record that can forge the next one, and the journal is an audit trail, not a log
sink. `Op` is a closed set with one token each, so a typo becomes `env.corrupt` rather than a new
operation nobody can search for.

**4. `ports::Fs` states the crash contract; only the adapter implements it.** The port's `write`
promises "after an interruption at any point the reader sees the previous bytes or the new ones,
never a partial file", and `adapters::fs_local` keeps that promise with `tmp/` → `fsync` →
`rename` → `fsync(dir)`. The port hands back `Kind` (five cases) rather than `std::io::ErrorKind`
because a decision layer that branches on operating-system vocabulary is a decision layer that
only works on one operating system. `list` returns sorted store-relative paths and excludes
`tmp/`: staging is not store content, so a leftover staging file can never be mistaken for a
blob.

**5. A path read from a record is untrusted input.** `Local::resolve` refuses anything that is not
a run of normal components — `..`, an absolute path, an empty string — with `Kind::NotAFile`.
The threat is not an attacker with a shell; it is a corrupted tree that sends a write outside the
store, which is the one way a store can damage something it does not own.

Consequences:

- **Content addressing is dedupe and verification for free**, and neither is code we wrote.
- **The store is testable without a disk for the pure part and with a real one for the contract.**
  `history` names no file; the crash contract is proved in `tests/integration` where a real
  filesystem can disagree with us.
- **Ordering is the caller's, and that is now visible.** `write` is atomic per file; the order
  across files (objects, then trees, then the entry, per `HISTORY.md` §7) belongs to the
  transaction that M6 introduces. Nothing in this milestone claims it.

Accepted debt, with the signal that says it has gone wrong:

- **No `lock`, no `inflight`, no drift scan.** They need a process identity and a transaction to
  guard, which arrive in M6. Until then `tmp/` is not swept at construction, and a test proves a
  leftover staging file is never trusted.
- **Verify-on-read is a convention with exactly one call site**: the acceptance test composes
  `fs.read` + `object::verify` by hand. M6 must funnel every read through one function, and the
  signal that it did not is a second hand-written `verify` call appearing anywhere. If that
  happens, the fix is a `Store` in `adapters/`, not another convention.
- **No `rename` on the port.** `ARCHITECTURE.md` lists it because undo materializes a tree by
  renaming into place; there is no caller until M8, and a port method with no caller is dead
  code that no lint will catch. It returns when undo does.

Alternatives rejected: a framed blob format (buys nothing, costs dedupe and a parse that can be
wrong); a binary tree format (unreviewable in a diff, and the store is meant to be inspectable);
storing the journal as one file (rewriting an append log is the thing an append log exists to
avoid); a `Fs` implementation per platform in `core/` (the dependency rule, and untestable).

Revisit if: the journal's file-per-entry layout becomes slow enough that `entries()` is a
measurable cost against `el log`'s budget — the fix is a cache, not a format change. Or if a
projection ever needs a file mode, which needs a tree field and a new ADR.

Refs: FR-33, FR-34; `HISTORY.md` §2–§4, §7; GATE.md §3; ADR-0002, ADR-0015
