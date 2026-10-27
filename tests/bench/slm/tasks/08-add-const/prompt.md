Add `pub const LIMIT: u32 = 10;` to `src/util.rs`. A const needs `--sig` because its type
is not inferable. The crate must compile.
