// `el.policy` is the project's opinion about what a change may cost: the file
// ceiling, the function budget, whether a preamble is required, which constructs
// are banned, which dependencies are allowed. L0 reads it, so a policy that is
// wrong has to be an error rather than a silent default — otherwise a typo
// quietly lowers a ceiling to zero or turns a required preamble off.
//
// Shape: `key = value`, one per line, `#` for comments. The defaults are the ones
// AGENTS.md names, so a project with no `el.policy` gets the same rules as this
// repository. Unknown keys are refused, because a misspelt key is a rule the user
// believes is in force and is not.
//
// It deliberately does not decide what `gate = check` *means* — that is the gate
// tier's question (M9) — and it does not read the file: text in, value out.

use std::fmt;

/// How a new body is stored. `auto` splits when the grammar can, `always` refuses
/// a body it cannot split, and `never` keeps the body opaque (FR-72).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Auto,
    Always,
    Never,
}

impl Split {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "auto" => Some(Self::Auto),
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

/// What a project allows a change to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Hard ceiling for any rendered source file, in lines.
    pub ceiling: usize,
    /// Budget for one element's body, in lines.
    pub max_fn_lines: usize,
    pub require_preamble: bool,
    /// Constructs that may not appear in a body (`forbid = unsafe`).
    pub forbid: Vec<String>,
    /// Dependencies a change may add. Empty means "none may be added".
    pub allowed_deps: Vec<String>,
    /// Verbatim, because the gate tier interprets it, not the policy.
    pub gate: String,
    pub split: Split,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            ceiling: 200,
            max_fn_lines: 15,
            require_preamble: true,
            forbid: vec!["unsafe".to_string()],
            allowed_deps: Vec::new(),
            gate: "check".to_string(),
            split: Split::Auto,
        }
    }
}

/// A policy file that cannot be obeyed, at the line that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub text: String,
    pub reason: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "el.policy line {}: {} (got {:?})",
            self.line, self.reason, self.text
        )
    }
}

const GATES: [&str; 4] = ["model", "check", "strict", "semantic"];

impl Policy {
    /// Read a policy file. Empty or absent text yields the defaults.
    ///
    /// Contract:
    ///   - Deterministic and total over the text: every rejection carries the
    ///     1-based line number and the offending text.
    ///   - A key that appears twice: the last one wins, because the file is read
    ///     top to bottom like a shell script.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut policy = Self::default();

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let content = raw.split('#').next().unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            let (key, value) = content.split_once('=').ok_or_else(|| Error {
                line,
                text: content.to_string(),
                reason: "expected `key = value`",
            })?;
            policy.set(key.trim(), value.trim(), line, content)?;
        }

        Ok(policy)
    }

    fn set(&mut self, key: &str, value: &str, line: usize, text: &str) -> Result<(), Error> {
        let bad = |reason: &'static str| Error {
            line,
            text: text.to_string(),
            reason,
        };
        match key {
            "ceiling" => self.ceiling = number(value).ok_or_else(|| bad("ceiling must be a number"))?,
            "max_fn_lines" => {
                self.max_fn_lines = number(value).ok_or_else(|| bad("max_fn_lines must be a number"))?
            }
            "require_preamble" => {
                self.require_preamble = matches!(value, "true" | "false")
                    .then_some(value == "true")
                    .ok_or_else(|| bad("require_preamble must be true or false"))?
            }
            "forbid" => self.forbid = list(value),
            "deps" => self.allowed_deps = list(value),
            "gate" => {
                if !GATES.contains(&value) {
                    return Err(bad("gate must be model, check, strict or semantic"));
                }
                self.gate = value.to_string();
            }
            "split" => {
                self.split = Split::parse(value)
                    .ok_or_else(|| bad("split must be auto, always or never"))?
            }
            _ => return Err(bad("unknown key")),
        }
        Ok(())
    }

    /// Whether a change may add `name` as a dependency.
    pub fn allows_dep(&self, name: &str) -> bool {
        self.allowed_deps.iter().any(|allowed| allowed == name)
    }

    /// Whether `word` may not appear in a body.
    pub fn forbids(&self, word: &str) -> bool {
        self.forbid.iter().any(|banned| banned == word)
    }
}

fn number(value: &str) -> Option<usize> {
    value.parse().ok()
}

/// `none` is the empty list; otherwise comma-separated, whitespace-trimmed.
fn list(value: &str) -> Vec<String> {
    if value == "none" {
        return Vec::new();
    }
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
