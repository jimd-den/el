// A logical statement of a body, from source text to the text the tree stores:
// the delimiters the writer left open, the terminator the renderer needs, and the
// refusal when a closer is wrong. It knows nothing about nesting; `grammar` owns
// the block structure and calls this as each physical line arrives.

use super::Error;
use crate::core::model::body::BodyStmt;
use crate::core::text::{balance, scan};

/// Whether a line's final `{` opens a block: only when everything before it is
/// complete. An open `(` or `[` in the prefix makes the line an expression that
/// continues, not a block.
pub(super) fn opens_block(text: &str) -> bool {
    match text.strip_suffix('{') {
        Some(prefix) => balance::balanced(prefix) == Some(true),
        None => false,
    }
}

/// A logical statement from the lines seen so far: `Some(text)` when its
/// delimiters are closed, `None` when an `(` or `[` is still open.
///
/// An unclosed `{` with no open `(`/`[` is not a statement and cannot become
/// one, so it is `Unbalanced`; a block opener is handled by `opens_block`.
pub(super) fn complete(text: &str, line: usize) -> Result<Option<String>, Error> {
    let expected = expected_closers(text, line)?;
    if expected.is_empty() {
        return Ok(Some(text.to_string()));
    }
    if expected.iter().any(|closer| matches!(closer, ')' | ']')) {
        return Ok(None);
    }
    Err(Error::Unbalanced { line })
}

/// The text with every unclosed `)` or `]` appended at the end; an unclosed `{`
/// that is not a block opener is refused.
pub(super) fn repair(text: &str, line: usize) -> Result<String, Error> {
    let expected = expected_closers(text, line)?;
    if expected.contains(&'}') {
        return Err(Error::Unbalanced { line });
    }
    let mut repaired = text.to_string();
    repaired.extend(expected.into_iter().rev());
    Ok(repaired)
}

/// The closers a statement still expects, in the order its openers appeared.
/// `Unterminated` when a literal or comment never closes; `Unbalanced` when a
/// closer does not match its opener.
fn expected_closers(text: &str, line: usize) -> Result<Vec<char>, Error> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut expected: Vec<char> = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        let c = chars[pos].1;
        let next = scan::char_at(&chars, pos + 1);
        match c {
            '/' if next == Some('/') => pos = scan::skip_line(&chars, pos),
            '/' if next == Some('*') => {
                pos = scan::skip_block_comment(&chars, pos).ok_or(until(line))?
            }
            '"' => pos = scan::skip_string(&chars, pos).ok_or(until(line))?,
            '\'' => pos = scan::skip_char_or_lifetime(&chars, pos).ok_or(until(line))?,
            'b' if next == Some('\'') => {
                pos = scan::skip_char_or_lifetime(&chars, pos + 1).ok_or(until(line))?
            }
            'r' | 'b' if scan::is_raw_string_start(&chars, pos) => {
                pos = scan::skip_raw_string(&chars, pos).ok_or(until(line))?
            }
            '{' | '(' | '[' => {
                expected.push(match c {
                    '{' => '}',
                    '(' => ')',
                    _ => ']',
                });
                pos += 1;
            }
            '}' | ')' | ']' => {
                if expected.pop() != Some(c) {
                    return Err(Error::Unbalanced { line });
                }
                pos += 1;
            }
            _ => pos += 1,
        }
    }

    Ok(expected)
}

fn until(line: usize) -> Error {
    Error::Unterminated { line }
}

/// True when the line brings an `else` branch, either alone or after `}`.
pub(super) fn is_else_branch(text: &str) -> bool {
    let rest = text.strip_prefix('}').unwrap_or(text);
    first_word(rest) == "else"
}

/// The leading identifier-like word, so `return(x)` and `return;` are both `return`.
fn first_word(text: &str) -> &str {
    let rest = text.trim_start();
    let end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Words that cannot end a body as a tail expression.
const TAIL_FORBIDDEN: &[&str] = &[
    "let", "use", "const", "static", "type", "return", "break", "continue",
];

/// Apply the semicolon policy to one sibling list.
pub(super) fn with_semicolons(mut siblings: Vec<BodyStmt>) -> Vec<BodyStmt> {
    let last = siblings.len().saturating_sub(1);
    for (index, stmt) in siblings.iter_mut().enumerate() {
        if needs_semicolon(&stmt.text, index == last) {
            stmt.text.push(';');
        }
    }
    siblings
}

/// A non-last statement needs a semicolon unless it already ends in `;`, `{`, `}`
/// or `,`; the last is a tail expression, except for the keywords above.
fn needs_semicolon(text: &str, is_last: bool) -> bool {
    // A comment or attribute is not an expression: a `;` after it would land
    // inside the comment or between the attribute and the item it decorates.
    let trimmed = text.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("#[")
        || trimmed.starts_with("#!")
    {
        return false;
    }
    let ends = matches!(text.chars().last(), Some(';' | '{' | '}' | ','));
    !ends && (!is_last || TAIL_FORBIDDEN.contains(&first_word(text)))
}
