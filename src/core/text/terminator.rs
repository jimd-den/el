// A function body is a sequence of statements, and `el` edits one at a time, so
// it needs positions it can prove are boundaries. This module is that policy:
// a delimiter stack plus two rules, and a refusal when neither can be applied.
//
//   - A `;` at depth zero ends a statement. Nothing else does.
//   - A `}` that empties the stack ends a statement, *unless* the next token
//     continues the expression it closes — `else`, `as`, or an operator such as
//     `.` in `match x { .. }.to_string()`. ADR-0013 calls these trailing
//     modifiers; they belong to the same statement, not to the next one.
//
// It deliberately does not infer Rust grammar: no notion of items, expressions,
// macros or generics. `None` means no boundary in this body is provable, and
// the body is stored opaque (FR-63, invariant I6).

use super::scan;
use super::scan::char_at;

/// Punctuation after a closing brace that continues the same statement.
const CONTINUATIONS: &[char] = &[
    '.', '?', ',', ')', ']', ':', '+', '-', '*', '/', '%', '=', '<', '>', '&', '|', '^',
];

/// Words after a closing brace that continue the same statement.
const CONTINUATION_WORDS: &[&str] = &["else", "as"];

/// Byte indices at which a statement may end, ascending.
///
/// Contract:
///   - Every returned index is the byte offset of an ASCII `;` or `}` in `body`.
///   - A `;` or `}` inside any literal, comment or delimiter pair is not one.
///   - `None` when a literal or comment never closes, or when a delimiter is
///     unmatched; the caller must then keep the body opaque.
pub(crate) fn statement_ends(body: &str) -> Option<Vec<usize>> {
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    let mut ends = Vec::new();
    let mut closers: Vec<char> = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        let (index, c) = chars[pos];
        let next = char_at(&chars, pos + 1);

        match c {
            '/' if next == Some('/') => pos = scan::skip_line(&chars, pos),
            '/' if next == Some('*') => pos = scan::skip_block_comment(&chars, pos)?,
            '"' => pos = scan::skip_string(&chars, pos)?,
            '\'' => pos = scan::skip_char_or_lifetime(&chars, pos)?,
            'r' | 'b' if scan::is_raw_string_start(&chars, pos) => {
                pos = scan::skip_raw_string(&chars, pos)?
            }
            '{' => {
                closers.push('}');
                pos += 1;
            }
            '(' => {
                closers.push(')');
                pos += 1;
            }
            '[' => {
                closers.push(']');
                pos += 1;
            }
            '}' | ')' | ']' => {
                if closers.pop()? != c {
                    return None;
                }
                pos += 1;
                if closers.is_empty() && c == '}' && !continues(&chars, pos) {
                    ends.push(index);
                }
            }
            ';' if closers.is_empty() => {
                ends.push(index);
                pos += 1;
            }
            _ => pos += 1,
        }
    }

    if closers.is_empty() { Some(ends) } else { None }
}

/// True when the token at `pos`, after any trivia, keeps the statement open.
fn continues(chars: &[(usize, char)], start: usize) -> bool {
    let mut pos = start;

    loop {
        match char_at(chars, pos) {
            Some(c) if c.is_whitespace() => pos += 1,
            Some('/') if char_at(chars, pos + 1) == Some('/') => pos = scan::skip_line(chars, pos),
            Some('/') if char_at(chars, pos + 1) == Some('*') => match scan::skip_block_comment(chars, pos) {
                Some(after) => pos = after,
                None => return true,
            },
            _ => break,
        }
    }

    match char_at(chars, pos) {
        None => false,
        Some(c) if c.is_alphanumeric() || c == '_' => {
            let mut end = pos;
            while char_at(chars, end).is_some_and(|c| c.is_alphanumeric() || c == '_') {
                end += 1;
            }
            let word: String = chars[pos..end].iter().map(|&(_, c)| c).collect();
            CONTINUATION_WORDS.contains(&word.as_str())
        }
        Some(c) => CONTINUATIONS.contains(&c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_top_level_semicolon() {
        assert_eq!(statement_ends("let a = 1;"), Some(vec![9]));
    }

    #[test]
    fn a_nested_block_ends_at_its_brace() {
        assert_eq!(statement_ends("if x { a(); }"), Some(vec![12]));
    }

    #[test]
    fn an_else_branch_continues_the_statement() {
        assert_eq!(statement_ends("if x { a(); } else { b(); }"), Some(vec![26]));
    }

    #[test]
    fn a_method_call_after_a_block_continues() {
        assert_eq!(statement_ends("match x { _ => 1 }.to_string();"), Some(vec![30]));
    }

    #[test]
    fn refuses_an_unclosed_brace() {
        assert_eq!(statement_ends("if x { a();"), None);
    }
}
