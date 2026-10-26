# Golden projection — two modules

Hand-written from the specification, never recorded from the renderer's output: a
golden recorded from a bug pins the bug.

What each line is here for:

| Line | Requirement |
|---|---|
| `// @el-projection …` | ADR-0009: the ownership contract, stated in the file |
| `pub mod greet;` | FR-15: no user writes `mod` lines. `pub` because `greet.rs` holds a `pub fn` — a private `mod` would make it unnameable, and `cargo check` said so (`E0603`) before this rule existed |
| `greet::greet();` | a generated `mod greet;` is a real namespace, so a call crosses it |
