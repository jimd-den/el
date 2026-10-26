// The ports are the interfaces the inner circles own and the outside world
// implements. Each one exists so that a decision can be tested without the thing
// it decides about: no disk, no subprocess, no clock, no terminal.
//
// Nothing here may name a concrete adapter. An implementation lives in
// `adapters/`, and `main.rs` is the only place that picks one.

pub mod clock;
pub mod fs;
pub mod tools;

#[cfg(test)]
pub mod fs_fake;

pub use clock::Clock;
pub use fs::{Fs, Kind};
pub use tools::{Finished, Tools};
