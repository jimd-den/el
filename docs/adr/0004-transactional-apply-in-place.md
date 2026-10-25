# 0004 — Transactional apply in place, not a shadow build

Status: accepted · Date: 2026-10-24
Context: the gate has to render the candidate tree and ask `cargo` about it before committing.
Two ways: render into a scratch directory and build there, or render in place and restore on
failure.

Decision: in place, with a lock, an `inflight` marker, and a content-addressed restore.
Sequence in `GATE.md` §3. `head` advances only after the gate passes.

Alternatives rejected:
- **Shadow directory.** Changes absolute source paths, which invalidates every `cargo`
  fingerprint: the gate would pay a full rebuild per edit (~10 s instead of ~0.5 s), which
  kills the iteration loop this tool is for. It also cannot see `build.rs` output, `cfg`
  paths, or a `.cargo/config.toml` `[build] target-dir` correctly.
- **Overlay filesystem.** A dependency on a kernel feature and permissions we cannot assume.
- **Optimistic write, no restore.** Leaves the tree broken on rejection, which is the one
  outcome the promise forbids.

Consequences: the tree is briefly the candidate version, so `el` must be the only writer
during a transaction (lock) and a crash must be recoverable (inflight + restore at next
invocation). The restore path is exercised by every rejected change, so it is the best-tested
code in the tool rather than the worst. `--dry-run` reuses the same path, so a preview is a
true preview, not an approximation.

Revisit if: a target environment makes in-place writes impossible (read-only checkouts) — then
a shadow mode becomes an explicit flag, with its rebuild cost stated.
