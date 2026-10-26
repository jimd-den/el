// `el` owns the braces, semicolons and parentheses of a body: a caller writes
// lines, and this module returns the tree the model stores, with the punctuation
// the renderer needs already decided.
//
// The grammar is line-oriented on purpose. A line ending in `{` with a balanced
// prefix opens a block and a line that is exactly `}` closes it; the renderer
// supplies one closing brace per opener, so a missing final brace is legal and a
// surplus one is not. A logical statement continues across lines while an `(` or
// `[` is open, keeping its newlines. It does not parse Rust: `else` branches
// cannot be represented, and are refused rather than guessed at.

use crate::core::model::body::{Body, BodyStmt};
use std::fmt;

use self::line::{complete, is_else_branch, opens_block, repair, with_semicolons};

mod line;

/// Something in a body that cannot become a tree the renderer can close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A literal or comment never closes, so no boundary on that line is provable.
    Unterminated { line: usize },
    /// A closer that does not match its opener, or a `{` that opens no block.
    Unbalanced { line: usize },
    /// A `}` line with no block open.
    UnexpectedClose { line: usize },
    /// An `else` branch cannot be attached to the block it follows.
    ElseAfterBlock { line: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unterminated { line } => {
                write!(f, "line {line}: unterminated literal or comment")
            }
            Self::Unbalanced { line } => write!(f, "line {line}: unbalanced delimiters"),
            Self::UnexpectedClose { line } => {
                write!(f, "line {line}: unexpected `}}` with no open block")
            }
            Self::ElseAfterBlock { line } => write!(
                f,
                "line {line}: `else` after a closing brace has no tree representation"
            ),
        }
    }
}

/// Parse a body into statements and their nested blocks.
///
/// Contract: blank lines are ignored, each line is trimmed, a line ending in `{`
/// with a balanced prefix owns the lines up to its `}`, and semicolons are
/// supplied per sibling list. A logical statement continues across lines while an
/// `(` or `[` is open; its newlines are kept in the statement text.
/// See the module preamble for what is refused.
pub fn parse(body: &str) -> Result<Body, Error> {
    let mut builder = Builder::default();
    let mut pending: Option<(String, usize)> = None;

    for (index, raw) in body.lines().enumerate() {
        let line = index + 1;
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        if let Some((mut buffer, start)) = pending.take() {
            buffer.push('\n');
            buffer.push_str(text);
            match complete(&buffer, line)? {
                Some(done) => builder.add(done),
                None => pending = Some((buffer, start)),
            }
            continue;
        }
        if text == "}" {
            builder.close(line)?;
        } else if is_else_branch(text) {
            return Err(Error::ElseAfterBlock { line });
        } else if opens_block(text) {
            builder.begin_block(text.to_string());
        } else if let Some(done) = complete(text, line)? {
            builder.add(done);
        } else {
            pending = Some((text.to_string(), line));
        }
    }

    if let Some((buffer, start)) = pending {
        builder.add(repair(&buffer, start)?);
    }
    Ok(Body {
        statements: line::attach_leading(builder.finish()),
    })
}

#[derive(Default)]
struct Builder {
    siblings: Vec<BodyStmt>,
    frames: Vec<(BodyStmt, Vec<BodyStmt>)>,
}

impl Builder {
    fn begin_block(&mut self, text: String) {
        let opener = BodyStmt {
            text,
            children: Vec::new(),
        };
        self.frames
            .push((opener, std::mem::take(&mut self.siblings)));
    }

    fn add(&mut self, text: String) {
        self.siblings.push(BodyStmt {
            text,
            children: Vec::new(),
        });
    }

    fn close(&mut self, line: usize) -> Result<(), Error> {
        let (mut opener, parent) = self.frames.pop().ok_or(Error::UnexpectedClose { line })?;
        opener.children = with_semicolons(std::mem::take(&mut self.siblings));
        self.siblings = parent;
        self.siblings.push(opener);
        Ok(())
    }

    /// Close every block still open, innermost first, then finish the root list.
    ///
    /// A missing final `}` is legal because the renderer supplies one brace per
    /// opener; only extra closers are an error.
    fn finish(mut self) -> Vec<BodyStmt> {
        while let Some((mut opener, parent)) = self.frames.pop() {
            opener.children = with_semicolons(self.siblings);
            self.siblings = parent;
            self.siblings.push(opener);
        }
        with_semicolons(self.siblings)
    }
}


#[cfg(test)]
#[path = "grammar_tests.rs"]
mod grammar_tests;

#[cfg(test)]
#[path = "grammar_comment_tests.rs"]
mod grammar_comment_tests;

#[cfg(test)]
#[path = "grammar_multiline_tests.rs"]
mod grammar_multiline_tests;
