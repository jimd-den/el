// Property tests over the whole model and the whole corpus: the invariants that
// must hold for every catalog and every body, not just the one in the example.
// Cargo needs a `main.rs` in a test subdirectory before it will run the files
// beside it, which is why this exists rather than one file per property.

mod catalog_round_trips;
mod split_round_trips;
