// What the user asked for, in the words `COMMANDS.md` §4 uses. No behaviour, no
// files, no decision: a `Command` is a sentence somebody typed, cleaned up.
//
// Everything is owned rather than borrowed, so a parsed command can outlive the
// `argv` it came from — `el apply` reads a script and then runs each line through
// the same vocabulary.

use crate::core::model::kind::Kind;
use crate::core::model::element::Visibility;

/// The universal flags, in the shape the rest of `el` reads them.
///
/// Defaults are stated rather than derived: `color` is on until a command says
/// otherwise, because only the terminal adapter can tell whether stdout is a TTY.
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

/// Where an item's text comes from: a file, or stdin to EOF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File(String),
}

impl Source {
    /// `--body -` means stdin; anything else is a path.
    pub fn parse(text: &str) -> Self {
        if text == "-" {
            Source::Stdin
        } else {
            Source::File(text.to_string())
        }
    }
}

/// `el add …`, as the model needs it.
///
/// `name` carries a module path for `add mod` and an identifier for every named
/// kind, because both are the one word the user typed first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Add {
    pub kind: Kind,
    pub name: String,
    /// `--in`: the file the item lives in. `None` means the crate root.
    pub module: Option<String>,
    pub visibility: Option<Visibility>,
    /// `--sig`: the whole declaration, keyword first.
    pub signature: Option<String>,
    pub body: Option<Source>,
    pub doc: Option<Source>,
    /// `--preamble`: a module's prose, which is a statement on the module element.
    pub preamble: Option<Source>,
    /// `--after`: insert after this id instead of appending.
    pub after: Option<String>,
    /// `--before`: insert before this id.
    pub before: Option<String>,
    /// `--at`: zero-based position among the siblings.
    pub at: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub id: String,
    pub rename: Option<String>,
    pub signature: Option<String>,
    pub body: Option<Source>,
    pub doc: Option<Source>,
    pub visibility: Option<Visibility>,
}

/// `el mv <subject> <module>`, or `el mv <stmt> --after <stmt>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mv {
    pub subject: String,
    pub module: Option<String>,
    pub after: Option<String>,
}

/// `el ls`: the catalog as a table, optionally narrowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ls {
    pub module: Option<String>,
    pub kind: Option<Kind>,
    pub opaque: bool,
}

/// `el tree`: the outline of the crate, with statements on request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub statements: bool,
}

/// `el show <id>`: one element, addressed by its permanent id only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Show {
    pub id: String,
    pub statements: bool,
    pub source: bool,
    pub history: bool,
    pub terse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    New { path: String, lib: bool, policy: bool },
    Init { force: bool },
    Add(Add),
    Edit(Edit),
    Rm { ids: Vec<String>, cascade: bool },
    Mv(Mv),
    Run { args: Vec<String> },
    Log { count: Option<usize>, attic: bool },
    Diff { a: Option<u64>, b: Option<u64> },
    Undo { n: Option<usize> },
    Redo,
    Revert { seq: u64 },
    Doctor { repair: bool, rebuild: bool },
    Ls(Ls),
    Tree(Tree),
    Show(Show),
    Version,
}

impl Command {
    /// The `cmd` field of the JSON envelope: the verb as typed, not the variant.
    pub fn name(&self) -> &'static str {
        match self {
            Command::New { .. } => "new",
            Command::Init { .. } => "init",
            Command::Add(_) => "add",
            Command::Edit(_) => "edit",
            Command::Rm { .. } => "rm",
            Command::Mv(_) => "mv",
            Command::Run { .. } => "run",
            Command::Log { .. } => "log",
            Command::Diff { .. } => "diff",
            Command::Undo { .. } => "undo",
            Command::Redo => "redo",
            Command::Revert { .. } => "revert",
            Command::Doctor { .. } => "doctor",
            Command::Ls(_) => "ls",
            Command::Tree(_) => "tree",
            Command::Show(_) => "show",
            Command::Version => "version",
        }
    }
}

/// A successfully parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub globals: Globals,
    pub command: Command,
}
