// argv → `Parsed`, against the one table of flags. Every rejection is a `Usage`
// carrying the code `COMMANDS.md` §5 publishes, so a script can branch on a failure
// without reading English.
//
// The table is read in both directions: `parse` asks "does this flag take a value",
// and `table.rs` prints the same answer. A flag that exists in one and not the
// other is a help text that lies, which is why both read `VALUE_FLAGS` here.

use super::request::{Add, Command, Edit, Globals, Mv, Parsed, Source};
use crate::core::model::kind::Kind;
use crate::core::model::element::Visibility;
use std::fmt;

/// Flags whose next argument is their value.
pub const VALUE_FLAGS: &[&str] = &[
    "--in", "--after", "--before", "--at", "--vis", "--sig", "--body", "--doc", "--attr",
    "--preamble", "--root", "--edition", "--body-mode", "--limit", "--module", "--kind", "-n",
];

/// A command line `el` refuses to understand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub code: &'static str,
    pub message: String,
}

impl Usage {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "el: {}", self.message)
    }
}

/// argv split into what it says and how it is modulated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    pub words: Vec<String>,
    /// Flags in the order they appeared, each with its value when it takes one.
    pub flags: Vec<(String, Option<String>)>,
    /// Everything after `--`, verbatim (`el run -- --nocapture`).
    pub rest: Vec<String>,
}

impl Args {
    /// Split `argv` without interpreting it.
    ///
    /// Contract: the first word is expected to be a verb; `--` ends flag parsing
    /// and everything after it is `rest`, untrimmed, because it belongs to
    /// another program.
    pub fn split(argv: &[String]) -> Result<Self, Usage> {
        let mut args = Args::default();
        let mut pending: Option<String> = None;
        let mut only_words = false;

        for token in argv {
            if only_words {
                args.rest.push(token.clone());
                continue;
            }
            if token == "--" {
                only_words = true;
                args.push_pending(&mut pending)?;
                continue;
            }
            if token.starts_with('-') && token.len() > 1 && !is_negative_number(token) {
                args.push_pending(&mut pending)?;
                if VALUE_FLAGS.contains(&token.as_str()) {
                    pending = Some(token.clone());
                } else {
                    args.flags.push((token.clone(), None));
                }
                continue;
            }
            if let Some(name) = pending.take() {
                args.flags.push((name, Some(token.clone())));
            } else {
                args.words.push(token.clone());
            }
        }
        args.push_pending(&mut pending)?;
        Ok(args)
    }

    fn push_pending(&mut self, pending: &mut Option<String>) -> Result<(), Usage> {
        match pending.take() {
            Some(name) => Err(Usage::new(
                "usage.missing_value",
                format!("{name} needs a value"),
            )),
            None => Ok(()),
        }
    }

    /// Whether a bare flag was given.
    pub fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|(flag, _)| flag == name)
    }

    /// A flag's value; a flag given twice is a usage error, not a last-one-wins.
    ///
    /// Last-one-wins is fine for a config file read top to bottom (`el.policy`
    /// does it) and wrong here: two `--sig` on one command line is a mistake the
    /// user cannot see, and silently honouring one of them hides it.
    pub fn value(&self, name: &str) -> Result<Option<String>, Usage> {
        let mut found = self
            .flags
            .iter()
            .filter(|(flag, _)| flag == name)
            .map(|(_, value)| value.clone());
        let first = found.next().flatten();
        if found.next().is_some() {
            return Err(Usage::new(
                "usage.repeated_flag",
                format!("{name} was given more than once"),
            ));
        }
        Ok(first)
    }

    fn required(&self, what: &str) -> Result<String, Usage> {
        self.words
            .first()
            .cloned()
            .ok_or_else(|| Usage::new("usage.missing_argument", format!("{what} needs a subject")))
    }
}

/// A token like `-1` is a value, not a flag.
fn is_negative_number(token: &str) -> bool {
    token[1..].chars().all(|c| c.is_ascii_digit()) && !token[1..].is_empty()
}

/// Parse a whole command line.
pub fn parse(argv: &[String]) -> Result<Parsed, Usage> {
    let args = Args::split(argv)?;
    let verb = args.words.first().cloned().unwrap_or_default();
    let globals = globals(&args)?;
    let command = match verb.as_str() {
        "new" => Command::New {
            path: args.required("new")?,
            lib: args.has("--lib"),
            policy: args.has("--policy"),
        },
        "init" => Command::Init {
            force: args.has("--force"),
        },
        "add" => Command::Add(add(&args)?),
        "edit" => Command::Edit(edit(&args)?),
        "rm" => Command::Rm {
            ids: args.words[1..].to_vec(),
            cascade: args.has("--cascade"),
        },
        "mv" => Command::Mv(Mv {
            subject: args.required("mv")?,
            module: args.value("--in")?.or(args.words.get(2).cloned()),
            after: args.value("--after")?,
        }),
        "run" => Command::Run {
            args: args.rest.clone(),
        },
        "log" => Command::Log {
            count: count(args.value("-n")?.as_deref())?,
        },
        "version" => Command::Version,
        "" => return Err(Usage::new("usage.missing_verb", "el needs a verb")),
        other => {
            return Err(Usage::new(
                "usage.unknown_verb",
                format!("{other:?} is not a verb; try `el skill`"),
            ));
        }
    };
    check_subject(&args, &command)?;
    Ok(Parsed { globals, command })
}

fn globals(args: &Args) -> Result<Globals, Usage> {
    Ok(Globals {
        json: args.has("--json"),
        quiet: args.has("--quiet"),
        dry_run: args.has("--dry-run"),
        yes: args.has("--yes"),
        fast: args.has("--fast"),
        strict: args.has("--strict"),
        semantic: args.has("--semantic"),
        verbose: args.has("--verbose"),
        no_color: args.has("--no-color"),
        root: args.value("--root")?,
    })
}

fn add(args: &Args) -> Result<Add, Usage> {
    let kind = args
        .words
        .get(1)
        .and_then(|word| Kind::parse(word))
        .ok_or_else(|| {
            Usage::new(
                "usage.unknown_kind",
                format!(
                    "add needs a kind, one of: {}",
                    Kind::all().iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" ")
                ),
            )
        })?;
    let name = args
        .words
        .get(2)
        .cloned()
        .ok_or_else(|| Usage::new("usage.missing_argument", format!("add {kind} needs a name")))?;

    Ok(Add {
        kind,
        name,
        module: args.value("--in")?,
        visibility: visibility(args.value("--vis")?)?,
        signature: args.value("--sig")?,
        body: source(args.value("--body")?),
        doc: source(args.value("--doc")?),
        preamble: source(args.value("--preamble")?),
        after: args.value("--after")?,
    })
}

fn edit(args: &Args) -> Result<Edit, Usage> {
    Ok(Edit {
        id: args.required("edit")?,
        rename: args.value("--rename")?,
        signature: args.value("--sig")?,
        body: source(args.value("--body")?),
        doc: source(args.value("--doc")?),
        visibility: visibility(args.value("--vis")?)?,
    })
}

fn visibility(value: Option<String>) -> Result<Option<Visibility>, Usage> {
    match value {
        None => Ok(None),
        Some(token) => Visibility::parse(&token).map(Some).ok_or_else(|| {
            Usage::new(
                "usage.bad_visibility",
                format!("--vis takes pub, crate or priv, not {token:?}"),
            )
        }),
    }
}

fn source(value: Option<String>) -> Option<Source> {
    value.map(|text| Source::parse(&text))
}

fn count(value: Option<&str>) -> Result<Option<usize>, Usage> {
    match value {
        None => Ok(None),
        Some(text) => text.parse().map(Some).map_err(|_| {
            Usage::new("usage.bad_value", format!("-n takes a count, not {text:?}"))
        }),
    }
}

/// Refuse stray subjects, so `el add fn f extra` is a usage error instead of a
/// silently ignored word.
fn check_subject(args: &Args, command: &Command) -> Result<(), Usage> {
    let expected = match command {
        Command::New { .. } | Command::Mv(_) => 2,
        Command::Add(_) => 3,
        Command::Edit(_) => 2,
        Command::Version | Command::Init { .. } => 1,
        Command::Rm { .. } | Command::Run { .. } | Command::Log { .. } => usize::MAX,
    };
    if args.words.len() > expected {
        return Err(Usage::new(
            "usage.unexpected_argument",
            format!("{} does not take {:?}", command.name(), args.words[expected]),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
