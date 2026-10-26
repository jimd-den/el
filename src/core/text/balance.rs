// A body with an unmatched `{` cannot be rendered into anything the compiler can
// even complain about usefully, so L0 asks this before it asks anything else
// (GATE.md §5, `model.unbalanced`). The question is only about delimiters, and it
// is asked of *code*, so the same skips that protect the splitter protect this:
// `"}"` in a string does not count, nor does a brace in a comment.
//
// Shape: one stack, three openers, three closers, and `None` when the text cannot
// be scanned at all (an unterminated literal), which is a different failure from
// "unbalanced" and is reported as such by the caller.
//
// It deliberately does not know which delimiters are *legal* where — `(` closing
// `[` is caught, but `fn f())` is the compiler's business, not ours.

use super::scan;

/// Whether every opener in `text` is closed by its own closer, in order.
///
/// Contract: text inside strings, raw strings, char literals, lifetimes, line
/// comments and block comments is skipped, so it never contributes a delimiter.
/// `None` when a literal or comment never closes.
pub fn balanced(text: &str) -> Option<bool> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut stack: Vec<char> = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        let c = chars[pos].1;
        let next = scan::char_at(&chars, pos + 1);

        match c {
            '/' if next == Some('/') => pos = scan::skip_line(&chars, pos),
            '/' if next == Some('*') => pos = scan::skip_block_comment(&chars, pos)?,
            '"' => pos = scan::skip_string(&chars, pos)?,
            '\'' => pos = scan::skip_char_or_lifetime(&chars, pos)?,
            'b' if next == Some('\'') => pos = scan::skip_char_or_lifetime(&chars, pos + 1)?,
            'r' | 'b' if scan::is_raw_string_start(&chars, pos) => {
                pos = scan::skip_raw_string(&chars, pos)?
            }
            '{' | '(' | '[' => {
                stack.push(c);
                pos += 1;
            }
            '}' | ')' | ']' => {
                match stack.pop() {
                    Some(open) if closes(open, c) => {}
                    // A closer with nothing open, or the wrong one: no repair
                    // exists that the writer did not intend, so stop asking.
                    _ => return Some(false),
                }
                pos += 1;
            }
            _ => pos += 1,
        }
    }

    Some(stack.is_empty())
}

fn closes(open: char, close: char) -> bool {
    matches!((open, close), ('{', '}') | ('(', ')') | ('[', ']'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_body_is_balanced() {
        assert_eq!(balanced("let v = vec![1, (2 + 3)];"), Some(true));
    }

    #[test]
    fn an_unclosed_block_is_not_balanced() {
        assert_eq!(balanced("if ready { apply(p);"), Some(false));
    }

    #[test]
    fn a_closer_that_does_not_match_its_opener_is_not_balanced() {
        assert_eq!(balanced("let v = (1];"), Some(false));
    }

    #[test]
    fn a_delimiter_inside_a_string_does_not_count() {
        assert_eq!(balanced(r#"log("{ unbalanced");"#), Some(true));
        assert_eq!(balanced(r#"log("}");"#), Some(true));
    }

    #[test]
    fn a_delimiter_inside_a_comment_does_not_count() {
        assert_eq!(balanced("// {\nplan();"), Some(true));
        assert_eq!(balanced("/* ) */ plan();"), Some(true));
    }

    #[test]
    fn a_brace_in_a_char_literal_does_not_count() {
        assert_eq!(balanced("let c = '{';"), Some(true));
        assert_eq!(balanced("let c = b'}';"), Some(true));
    }

    #[test]
    fn an_unterminated_literal_is_not_an_answer() {
        assert_eq!(balanced("log(\"open);"), None);
        assert_eq!(balanced("/* open"), None);
    }
}
