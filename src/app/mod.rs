// The application layer: the use cases, and the only place that knows the order a
// command's steps happen in. It reads ports and returns a `Report`; it never writes
// to a terminal, never names an adapter, and never exits the process.
//
// Shape: `run(argv, &Env) -> Report`, where `Env` is the composition root's gift of
// ports. Every failure is a value (`Report::error`), so the process exit code is
// decided once, at the edge, from data a test can read.
//
// It deliberately does not decide *how* output looks: the human and JSON renderers
// are adapters reading the same `Report`, which is what keeps `--json` from being a
// second code path through every verb.

pub mod exit;
pub mod report;

pub use exit::Exit;
pub use report::{Changed, Error, Gate, Report};
