# 0007 — The network is a subprocess, never a library

Status: accepted · Date: 2026-10-24
Context: `el search` and `el doc` need HTTPS. A TLS stack in-process is either `rustls` +
`ring` (a large dependency graph, C or assembly, a supply-chain surface) or an
implementation we should not write. Neither is acceptable for a 1.5 MB tool whose main job is
something else.

Decision: fetch by invoking a system tool — `$EL_FETCH`, default `curl -fsSL --max-time 20`.
The response is cached by URL digest for 7 days. `--offline` never spawns it. If no fetcher
exists, the command exits 4 with one actionable line and everything else still works.

Alternatives rejected: in-process TLS (dependency weight); `nc` (no TLS); HTTP only (docs are
HTTPS only); bundling a docs snapshot (tens of MB, stale immediately).

Consequences: search is the only capability that needs a tool the toolchain does not already
bring (`cargo` and `rust-analyzer` are part of a Rust install; `curl` is usually there but not
guaranteed). This is why search is M11 and the first thing cut. It is also why the cache is
aggressive: the common case (looking up `Vec::retain` for the fifth time) never touches the
network.

Revisit if: `std` ever grows an HTTP client, or a target environment has no fetcher at all —
the fallback there is a shipped index file, not a TLS dependency.
