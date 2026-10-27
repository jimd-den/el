// argv split into what it says and how it is modulated: words, flags, and everything
// after `--`. Knowing which flags take a value is the one table this layer owns, and
// `parse` reads it too — a flag that took a value in one file and not the other would
// be a parser that disagrees with itself.
//
// It deliberately does not interpret a flag's meaning. `--body -` is `Args`'s business
// only as far as "--body takes a value, and the value is -"; that `-` means stdin is
// the request vocabulary's word, one file over.

use std::fmt;

/// Flags whose next argument is their value.
pub const VALUE_FLAGS: &[&str] = &[
    "--in", "--after", "--before", "--at", "--vis", "--sig", "--body", "--doc", "--attr",
    "--preamble", "--root", "--edition", "--body-mode", "--limit", "--module", "--kind",
    "--rename", "-n", "--takes", "--returns", "--type", "--value",
];

/// Flags that stand alone. Anything not here and not in `VALUE_FLAGS` is refused,
/// because a flag the parser quietly ignores is a promise the command line breaks.
pub const BOOL_FLAGS: &[&str] = &[
    "--json", "--quiet", "--dry-run", "--yes", "--no-color", "--fast", "--strict",
    "--semantic", "--verbose", "--lib", "--policy", "--force", "--cascade", "--attic",
    "--repair", "--rebuild", "--statements", "--source", "--history", "--terse",
    "--opaque", "--help",
];

/// A command line `el` refuses to understand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub code: &'static str,
    pub message: String,
}

impl Usage {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
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

/// argv, split but not yet interpreted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    pub words: Vec<String>,
    /// Flags in the order they appeared, each with its value when it takes one.
    pub flags: Vec<(String, Option<String>)>,
    /// Everything after `--`, verbatim (`el run -- --nocapture`).
    pub rest: Vec<String>,
}

impl Args {
    /// Split `argv`.
    ///
    /// Contract: the first word is expected to be a verb; `--` ends flag parsing and
    /// everything after it is `rest`, untrimmed, because it belongs to another
    /// program.
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
        for (name, _) in &args.flags {
            if !VALUE_FLAGS.contains(&name.as_str()) && !BOOL_FLAGS.contains(&name.as_str()) {
                return Err(Usage::new(
                    "usage.unknown_flag",
                    format!("{name:?} is not a flag"),
                ));
            }
        }
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

    /// A flag's value. A flag given twice is a usage error, not a last-one-wins.
    ///
    /// Last-one-wins is right for a config file read top to bottom (`el.policy`) and
    /// wrong here: two `--sig` on one command line is a mistake the user cannot see,
    /// and silently honouring one of them hides it.
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

    /// The word after the verb: the subject every subject-taking verb consumes.
    ///
    /// `words[0]` is the verb, so reading `first()` here is how `el new hello`
    /// once parsed its path as `"new"`. The index is the contract.
    pub fn required(&self, what: &str) -> Result<String, Usage> {
        self.words
            .get(1)
            .cloned()
            .ok_or_else(|| Usage::new("usage.missing_argument", format!("{what} needs a subject")))
    }
}

/// A token like `-1` is a value, not a flag.
fn is_negative_number(token: &str) -> bool {
    !token[1..].is_empty() && token[1..].chars().all(|c| c.is_ascii_digit())
}
