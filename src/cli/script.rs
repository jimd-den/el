// A script is one change per line, in the same words `argv` uses. Comments start
// with `#` at a word boundary, single quotes are literal, and double quotes take
// backslash escapes so an `--sig` can carry spaces without a shell.
//
// It deliberately does not interpret the verbs: `apply` runs each line through
// `cli::parse`, so a script and a command line cannot disagree about a flag.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "apply line {}: {}", self.line, self.message)
    }
}

pub fn parse(text: &str) -> Result<Vec<Vec<String>>, Error> {
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if let Some(words) = one(line, index + 1)? {
            lines.push(words);
        }
    }
    Ok(lines)
}

fn one(line: &str, number: usize) -> Result<Option<Vec<String>>, Error> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '#' if !in_word => break,
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => word.push(ch),
                        None => return Err(unterminated(number)),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => {
                            let escaped = chars.next().ok_or_else(|| unterminated(number))?;
                            word.push(match escaped {
                                'n' => '\n',
                                't' => '\t',
                                other => other,
                            });
                        }
                        Some(ch) => word.push(ch),
                        None => return Err(unterminated(number)),
                    }
                }
            }
            '\\' => {
                in_word = true;
                word.push(chars.next().ok_or_else(|| unterminated(number))?);
            }
            other => {
                in_word = true;
                word.push(other);
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Ok((!words.is_empty()).then_some(words))
}

fn unterminated(line: usize) -> Error {
    Error {
        line,
        message: "unterminated quote or escape".to_string(),
    }
}

#[cfg(test)]
#[path = "script_tests.rs"]
mod script_tests;
