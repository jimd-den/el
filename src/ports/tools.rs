// Running another program, as a port. `cargo` and `rust-analyzer` are subprocesses
// and nothing else: no library bindings, no async runtime, no SDK. That decision is
// ADR-0004's, and this trait is where it becomes checkable — the app can ask for a
// program to run, and a test can answer without one existing (FR-14's gate tiers
// arrive in M9; `el run` needs it now).
//
// Output is captured rather than inherited, because a report that says what happened
// is the only output `el` promises; a child that writes straight to the terminal
// would escape it. `stdin` is the one exception a body needs (`--body -`).
//
// It deliberately does not interpret exit codes. `cargo` says 101 for a compile
// failure and 1 for a bad flag, and which of those matters is the gate's question,
// not the port's.

use std::fmt;

/// A finished child process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    /// The raw status code, or -1 when the child was killed by a signal.
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// True when the run ended by timeout rather than by exiting: the child
    /// was killed, so the code above is not a verdict.
    pub timed_out: bool,
}

impl Finished {
    pub fn succeeded(&self) -> bool {
        self.code == 0
    }
}

/// Why a program could not be run at all — never why it failed, which is its own
/// exit code and its own diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub program: String,
    /// Whether the program is missing, which is `env.missing_tool` and exit 4.
    pub missing: bool,
    pub detail: String,
}

impl Error {
    pub fn missing(program: &str) -> Self {
        Self {
            program: program.to_string(),
            missing: true,
            detail: "not found on PATH".to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.missing {
            write!(f, "el: {} is not installed ({}); install it or use --fast", self.program, self.detail)
        } else {
            write!(f, "el: could not run {}: {}", self.program, self.detail)
        }
    }
}

/// A program to run, its arguments, and what to feed it.
pub trait Tools {
    fn run(&self, argv: &[String], stdin: Option<&str>) -> Result<Finished, Error>;

    /// Like `run`, but the child is killed after `timeout_secs` and the
    /// result reports `timed_out` instead of an exit code. The default
    /// implementation has no timeout to enforce — it is the test double's
    /// shape — so it delegates to `run`; the real runner implements it.
    fn run_timed(
        &self,
        argv: &[String],
        stdin: Option<&str>,
        _timeout_secs: u64,
    ) -> Result<Finished, Error> {
        self.run(argv, stdin)
    }
}
