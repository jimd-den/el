# Archive — `AGENTS.md` §0 as it stood before `el` (ỌKỌ)

Preserved verbatim by ADR-0010. The table below is the `PROJECT`-bearing part of §0; every
command, path and budget in §0 is unchanged and still governs this repository.

| Slot | Value |
|---|---|
| `PROJECT` | ỌKỌ is a console that replaces the display server with a display list, so a bare machine reaches a login prompt and hot-editable panels with no browser, GPU, or network. |
| `LANGUAGE` / `STACK` | Rust 1.99.1, edition 2024 / `std` + `libc` + `skrifa`; no async runtime, no GUI toolkit, no browser in the boot path |

If `el` later lives beside ỌKỌ in one repository, §0 becomes a nested table with one row per
crate and this file is the provenance for the ỌKỌ row.
