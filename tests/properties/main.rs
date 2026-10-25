// Property tests over the whole model: the invariants that must hold for every
// catalog, not just the one in the example. Cargo needs a `main.rs` in a test
// subdirectory before it will run the files beside it, which is why this exists.

mod catalog_round_trips;
