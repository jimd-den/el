# History — catalog, journal, snapshots, rollback

> Promise `P2`: every mutation is reversible, exactly. This document says what "exactly"
> means, what a snapshot costs, and what happens when the machine dies mid-write.

---

## 1. Why not just call `git`

`git` is present on most developer machines and already solves snapshots, diffs and undo.
Rejected because:

1. **`git` is not guaranteed, and its absence would break the core promise.** `el` must work
   on a bare machine with `cargo` and nothing else. `curl` is optional (search), `rust-analyzer`
   is optional (semantics), but history is not optional.
2. **A commit per keystroke-scale edit pollutes the user's history.** `el add stmt` produces
   dozens of entries per session; those are not commits, they are *edits*. Making them git
   commits forces the user to rebase their own work around a tool's bookkeeping.
3. **We need the tree digest before the write, not after.** The transaction restores from a
   snapshot it created three milliseconds ago; that is a store operation, not a git operation.

What we lose: packing, deltas, remotes, merge, `git log --graph`. Content-addressing gives
deduplication and integrity; the rest is deferred (`§9`). If the user wants git, the
projection is a normal working tree — `git add -A && git commit` after an `el` session is
their choice, and `el log` is not affected by it.

`el init` writes `.el/` into `.gitignore` and never touches `.git/`.

---

## 2. Layout

```text
.el/
  version            "el1\n"                      format version, one line
  head               "42\n"                       current sequence number
  catalog            derived; rebuildable from the journal
  journal/000042     one file per entry
  attic/000043       entries whose future was truncated by undo + new change
  trees/<ab>/<cdef>  one file per tree (the whole projection)
  objects/<ab>/<cdef>one file per blob (one projection file)
  diags/<ab>/<cdef>  one file per baseline (GATE.md §4.2)
  log/ra.log         quarantined rust-analyzer stderr (rotated once at 256 KB)
  lock               pid + start time, removed on release
  inflight           present only during a transaction
  tmp/               staging for atomic rename; emptied on start
```

Two characters of digest prefix, like git: `objects/3f/9a1c…`. Endianness of the split does
not matter; consistency does.

---

## 3. Content addressing

Digest: **SHA-256**, implemented in `core/history/sha256.rs` (std only, ~110 lines, tested
against NIST vectors). Why not a cheaper hash: the store must detect corruption, and a
64-bit non-cryptographic hash at 10⁵ objects has a birthday collision probability that is
not zero. SHA-256 is also what every other tool expects if this ever needs to interop.

| Rule | Reason |
|---|---|
| Digests are verified on **every read** (`FR-33`) | Silent corruption becomes a named error (`env.corrupt`), not a wrong restore |
| A blob's path is its digest | Dedupe is free; two identical files are stored once |
| Content is never rewritten | Append-only store; only `doctor --prune` deletes |
| Objects are written before the journal entry that references them | A journal entry never points at a missing blob |

---

## 4. Record formats

All line-oriented, tab-separated, no embedded newlines (`SRD.md` §10). Free text is
sanitized on write (`\t`, `\n`, `\r` → spaces) so a record is always one line.

### 4.1 Journal entry — `.el/journal/000042`

```text
el1	42	1761266000	agent	edit	el-0007	e4b1…	9f2c…	ok:0/1/5	rename apply to apply_change
```

| Field | Meaning |
|---|---|
| `el1` | Format tag. A future el2 must refuse to write el1 and must be able to read it |
| `seq` | Monotonic, zero-padded to 6 in the path only |
| `ts` | Unix seconds, from the `Clock` port (injected; tests use a fixed clock) |
| `actor` | `human`, `agent`, `ci`, or `$EL_ACTOR`; free-form but sanitized |
| `op` | `init`, `add`, `edit`, `rm`, `mv`, `dep`, `policy`, `adopt`, `import`, `revert`, `repair` |
| `target` | Element id, module path, dep name, or `-` for batch/repair |
| `from`, `to` | Tree digests: what it was, what it became |
| `gate` | `ok:new/fixed/existing`, `rejected:<n>`, `preview:<n>`, `skip`, `model`, `ra` |
| `msg` | Human sentence, ≤ 200 chars, sanitized |

A batch (`el apply`) is one entry, one target `-`, and the message lists the change count,
so history stays at the granularity of *intent*, not of *keystroke* (`FR-10`).

### 4.2 Tree — `.el/trees/<ab>/<cdef>`

```text
src/main.rs	3f9a…
src/core/plan.rs	9f2c…
Cargo.toml	71b0…
```

Sorted by path. The tree is the definition of "the projection": what `el` owns, what it
restores, what it diffs. Anything not listed is outside the projection and is never touched
by undo (`target/`, `.git/`, `README.md`, a user's own files).

### 4.3 Catalog — `.el/catalog`

The derived, human-greppable current model. TSV, one record per line, sorted by id:

```text
E	el-0007	fn	core::plan::apply	src/core/plan.rs	pub	split	9
S	el-0102	stmt	-	el-0007	2	-	let change = self.clamp();
M	src/core/plan.rs	preamble	12
D	serde@1.0	allowed	"why: json export"
P	ceiling	200
```

`E` element, `S` statement, `M` module, `D` dependency, `P` policy. It is a **cache**:
`doctor --rebuild` regenerates it from the journal and the store, and a test asserts the
regenerated bytes equal the on-disk bytes (`FR-34`). If they differ, the cache is the thing
that is wrong, and `el` says so rather than silently trusting it.

---

## 5. Identity

| Id | Assigned | Lifetime |
|---|---|---|
| `el-NNNN` | At creation, from a counter in the catalog | Permanent. Survives rename, move, body-mode change, adopt |
| alias `<module>::<name>` | Derived | Changes with the module and the name; unique at all times (`FR-2`) |
| statement ids | At split, in body order | Stable while the statement exists; reordering keeps the id, so history reads as a move |

The counter never resets, never reuses a number, and is stored in the catalog as `next_id`.
Deleting `el-0011` does not make `el-0011` available again — a recycled id would make old
journal entries lie.

---

## 6. Undo, redo, revert

```text
head := 42    journal: … 41 42         attic: —

el undo           head := 41, tree 41 materialized.  redo available.
el redo           head := 42, tree 42 materialized.  redo unavailable.
el undo 3         head := 39.
el revert 12      append entry 43 whose tree == tree(12).  History is never rewritten.
el undo; el add fn x
                  entries 41.. were the future → moved to attic/, head := 41, append 42'.
                  `el log --attic` still shows them; `el redo` is unavailable.
```

- **Undo is a head move plus a materialize.** It is not a reverse diff, so it cannot
  half-apply (`FR-29`).
- **Redo is available until a new mutation occurs** (`FR-30`). This is the editor rule, chosen
  because the alternative — a DAG with branches — has a UI cost (branch names, merge,
  conflict) that v1 cannot pay. Future entries are preserved in `attic/`, so nothing is
  destroyed, only deselected.
- **Revert appends.** A revert that rewrote history would make the journal unable to explain
  how the tree got where it is (`FR-31`).
- **Undo materializes the full tree**, including deletions: a path in head that is not in the
  target tree is unlinked. This is why the tree is the definition of the projection.

`el log` is the audit trail. Every entry's gate verdict is preserved, including rejections,
so a later reader can see that `el-0007` was once attempted with a type error and corrected.

---

## 7. Crash, lock, repair

Crash safety rests on ordering, not on luck:

```text
write inflight  →  write tree+entry (fsync)  →  write files (tmp+fsync+rename)  →  gate
                                                                                   ↓
                                                              head advances only here
```

- `.el/inflight` contains `seq, from, started, pid`. Its presence means "a transaction was
  interrupted after the point of no return".
- The next invocation (any command, before anything else) restores `head`'s tree and reports
  it on stderr once. `el doctor --repair` does the same explicitly and reports what it found.
- `.el/lock` is `O_CREAT|O_EXCL` with `pid` and start time. A lock whose pid is dead, or older
  than 10 minutes, is reclaimed with a note (`FR-39`). Two `el` processes on one crate is a
  user error, not a data-loss event: the loser exits 5 and writes nothing.
- `tmp/` is emptied at startup; a leftover tmp file is never trusted.

The invariant that makes this sufficient (`I3`): **the store is append-only and the tree is
written before the files.** At any crash point, either the new tree is fully recorded (so it
can be materialized) or it is not (so head still names the old one).

---

## 8. Space, and pruning

A 30-module crate is ~120 KB of source. Every snapshot shares blobs, so a typical
statement edit costs one new blob (~200 bytes) plus a tree (~1.5 KB). Ten thousand edits is
tens of megabytes — acceptable to keep forever, which is the default (`D-2`).

`el doctor --prune [--keep N]` is the escape hatch, with two rules: it never deletes a blob
reachable from a kept tree, and it reports exactly what it removed. Pruning is never
automatic; a store that shrinks itself is a store whose history you cannot cite.

---

## 9. Deferred, and what keeps it cheap

| Deferred | Kept open by |
|---|---|
| Branches / merge | Entries already carry `from`; a multi-parent field is additive |
| Push/pull / remotes | Trees and objects are content-addressed; any transport is an adapter |
| Tags / named states | One line in the catalog; `revert <tag>` resolves to a seq |
| `git` interop (`el export`) | The projection is a plain working tree; export is a copy |
| Per-element history view | Journal `target` is already an element id; `el show --history` is a filter |
