// The exit codes `COMMANDS.md` §2 publishes. They are a contract: a script branches
// on the number, so the number is named here once and never written as a literal in
// a `return`.
//
// The enum is exhaustive on purpose. A new failure that does not fit an existing
// code is a decision, and a decision is something to write down, not to bury in a
// `7`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Done, including a no-op (`changed: []`).
    Ok,
    /// Bad flags, unknown verb, missing argument. Nothing was read or written.
    Usage,
    /// L0 rejected the change. Nothing was written (`FR-18`).
    Violation,
    /// A gate tier rejected the change; the tree was restored (`FR-21`).
    Gate,
    /// A missing tool, drift needing `--adopt`, or an offline miss (`FR-42`).
    Env,
    /// A lock was held, or a concurrent modification was detected (`FR-39`).
    Conflict,
    /// Unknown element, module, alias or sequence.
    NotFound,
    /// A bug: the one code that means "report this".
    Internal,
}

impl Exit {
    pub fn code(self) -> u8 {
        match self {
            Exit::Ok => 0,
            Exit::Usage => 1,
            Exit::Violation => 2,
            Exit::Gate => 3,
            Exit::Env => 4,
            Exit::Conflict => 5,
            Exit::NotFound => 6,
            Exit::Internal => 7,
        }
    }

    /// Whether the command did what it was asked. The only test the edge needs.
    pub fn succeeded(self) -> bool {
        self == Exit::Ok
    }
}
