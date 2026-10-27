// Integration tests: real boundaries. Here the real `cargo` judges rendered
// output, because "valid Rust" is not a claim a unit test can make; and the real
// filesystem judges the store, because a crash contract is about bytes on a disk.

mod builders;
mod fs_write_is_atomic;
mod helpers;
mod rendered_crates_compile;
mod store_round_trip;
mod gate_latency;
mod restore_skips_manifest;
