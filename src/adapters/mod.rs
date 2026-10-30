// Adapters: where the outside world appears. Each one implements a port from
// `ports/`, and each one is the only place a fact about an operating system is
// allowed to live. `main.rs` picks them; nothing else names them.

pub mod clock_system;
pub mod fs_local;
pub mod lock;
pub mod out_human;
pub mod out_json;
mod out_json_fields;
pub mod store;
pub mod tools_proc;

pub use clock_system::SystemClock;
pub use fs_local::Local;
pub use store::Store;
pub use tools_proc::ProcTools;
