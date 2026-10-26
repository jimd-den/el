// Adapters: where the outside world appears. Each one implements a port from
// `ports/`, and each one is the only place a fact about an operating system is
// allowed to live. `main.rs` picks them; nothing else names them.

pub mod fs_local;

pub use fs_local::Local;
