// The universal flags, in the shape the rest of `el` reads them. They travel with
// every command, so they are one value rather than nine parameters.
//
// Defaults are stated rather than derived: `color` is on until a command says
// otherwise, because only the terminal adapter can tell whether stdout is a TTY.

/// `--json`, `--quiet`, the gate overrides, and `--root`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Globals {
    pub json: bool,
    pub quiet: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub fast: bool,
    pub strict: bool,
    pub semantic: bool,
    pub verbose: bool,
    pub no_color: bool,
    /// `--root`: run against another crate instead of the current directory.
    pub root: Option<String>,
}

impl Globals {
    /// The colour decision `el` passes to the terminal adapter. `--json` implies
    /// no colour (`COMMANDS.md` §1), so the flag cannot be forgotten at a call site.
    pub fn color(&self) -> bool {
        !self.no_color && !self.json
    }
}
