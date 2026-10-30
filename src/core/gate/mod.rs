// The gate's core: diagnostics, fingerprints, deltas, baselines. Pure — no I/O,
// no processes — so the verdict is a value a unit test can hold in its hand.
// The application layer (`app::transaction::gate`) owns running cargo and the
// store; this module owns deciding what cargo meant.

pub mod baseline;
pub mod delta;
pub mod diagnostics;
pub mod parse;

pub use baseline::{decode as decode_baseline, encode as encode_baseline};
pub use delta::{Delta, delta, verdict};
pub use diagnostics::{Diagnostic, Level};
pub use parse::{parse_cargo_json, Parsed};

/// A gate tier. L3 (`semantic`, rust-analyzer) arrives in M10 and is not a
/// variant here: naming it would promise a tier nothing runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// L0: the plan's model checks, already run before staging.
    Model,
    /// L1: `cargo check --all-targets --message-format=json`.
    Check,
    /// L2: L1 plus clippy `-D warnings` plus `cargo test --no-run`.
    Strict,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Check => "check",
            Self::Strict => "strict",
        }
    }
}
