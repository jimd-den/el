// What the user asked for, in the words `COMMANDS.md` §4 uses. No behaviour, no
// files, no decision: a `Command` is a sentence somebody typed, cleaned up.
//
// Everything is owned rather than borrowed, so a parsed command can outlive the
// `argv` it came from — `el apply` reads a script and then runs each line through
// the same vocabulary.

pub use super::globals::Globals;

use crate::core::model::kind::Kind;
use crate::core::model::element::Visibility;

/// Where an item's text comes from: a file, stdin to EOF, or a literal
/// carried by the caller (a batch item's body, which never touches a file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File(String),
    Literal(String),
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
    /// Plain-words flags, assembled into a signature by `head::signature`.
    /// `--takes` and `--returns` belong to `fn` and `test`; `--type` and
    /// `--value` belong to `const`, `static` and `type`.
    pub takes: Option<String>,
    pub returns: Option<String>,
    pub decl_type: Option<String>,
    pub value: Option<String>,
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
    /// `--body-mode split|opaque`: how the new body is stored. `None` means the
    /// project policy decides. Parsed here, honoured in `change::edit`.
    pub mode: Option<crate::core::plan::policy::Split>,
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

/// `el find <query>`: named elements by bare name, alias, or module path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Find {
    pub query: String,
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
    New {
        path: String,
        lib: bool,
        policy: bool,
        edition: Option<String>,
    },
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
    Find(Find),
    Explain { id: String },
    Check { scope: Option<String> },
    Test { target: Option<String>, all: bool, scope: Option<String>, impacted: bool },
    Fmt,
    Lint { scope: Option<String> },
    Status,
    Discover { query: String, limit: usize, cursor: usize },
    Read { reference: String, rev: Option<String>, lines: Option<(usize, usize)> },
    Tdd { phase: String, target: Option<String> },
    Batch,
    Apply { file: Option<String> },
    Skill,
    Help { verb: Option<String> },
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
            Command::Find(_) => "find",
            Command::Explain { .. } => "explain",
            Command::Check { .. } => "check",
            Command::Test { .. } => "test",
            Command::Fmt => "fmt",
            Command::Lint { .. } => "lint",
            Command::Status => "status",
            Command::Discover { .. } => "discover",
            Command::Read { .. } => "read",
            Command::Tdd { .. } => "tdd",
            Command::Batch => "batch",
            Command::Apply { .. } => "apply",
            Command::Skill => "skill",
            Command::Help { .. } => "help",
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
