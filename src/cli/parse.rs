// argv → `Parsed`, against the one table of flags. Every rejection is a `Usage`
// carrying the code `COMMANDS.md` §5 publishes, so a script can branch on a failure
// without reading English.
//
// The table is read in both directions: `parse` asks "does this flag take a value",
// and `table.rs` prints the same answer. A flag that exists in one and not the
// other is a help text that lies, which is why both read `VALUE_FLAGS` here.

use super::args::{Args, Usage};
use super::request::{Add, Command, Edit, Globals, Mv, Parsed, Source};
use crate::core::model::element::Visibility;
use crate::core::model::kind::Kind;

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
        other => match super::parse_read::command(other, &args)? {
            Some(command) => command,
            None => {
                return Err(Usage::new(
                    "usage.unknown_verb",
                    format!("{other:?} is not a verb; try `el skill`"),
                ));
            }
        },
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
        Command::New { .. } => 2,
        Command::Mv(_) if args.has("--in") => 2,
        Command::Mv(_) => 3,
        Command::Add(_) => 3,
        Command::Edit(_) | Command::Show(_) => 2,
        Command::Ls(_) | Command::Tree(_) | Command::Version | Command::Init { .. } => 1,
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
#[path = "parse_read_tests.rs"]
mod parse_read_tests;

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
