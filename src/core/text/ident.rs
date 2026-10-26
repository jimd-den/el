// A rename is a text edit, and a text edit that does not know where code starts
// and ends corrupts what the user wrote: `"apply"` in a message is not a
// reference to `fn apply`, and neither is `'apply` (a lifetime), nor the word in
// a comment. This module answers one question — where are the identifiers that
// are actually code — so that `plan::refs` can rewrite without guessing.
//
// It reuses the scanner's skips, so it inherits their refusal: an unterminated
// literal or comment means no position in this body is provable, and `None` is
// the only honest answer (the same refusal as `split`, for the same reason).
//
// It deliberately does not know Rust's keyword list, resolve paths, or decide
// what a name refers to. `a::b` yields two identifiers; which one is a type and
// which is a module is the compiler's business.

use super::scan;
use super::scan::char_at;

/// Byte offset and text of every identifier in code position, in source order.
///
/// Contract:
///   - Offsets are byte offsets into `body`, so `&body[offset..offset + len]` is
///     the identifier.
///   - An identifier starts with a letter or `_` and continues with letters,
///     digits or `_`; anything inside a string, raw string, char literal,
///     lifetime, line comment or block comment is not returned.
///   - `None` when a literal or comment never closes.
pub fn idents(body: &str) -> Option<Vec<(usize, &str)>> {
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    let mut found = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        let (index, c) = chars[pos];
        let next = char_at(&chars, pos + 1);

        match c {
            '/' if next == Some('/') => pos = scan::skip_line(&chars, pos),
            '/' if next == Some('*') => pos = scan::skip_block_comment(&chars, pos)?,
            '"' => pos = scan::skip_string(&chars, pos)?,
            '\'' => pos = scan::skip_char_or_lifetime(&chars, pos)?,
            // `b'x'` is a byte literal: the `b` belongs to it, not to a name.
            // Ordered before the raw-string arm so `b'` is not read as `b` + char.
            'b' if next == Some('\'') => pos = scan::skip_char_or_lifetime(&chars, pos + 1)?,
            'r' | 'b' if scan::is_raw_string_start(&chars, pos) => {
                pos = scan::skip_raw_string(&chars, pos)?
            }
            c if is_start(c) => {
                let start = pos;
                pos += 1;
                while pos < chars.len() && is_continue(chars[pos].1) {
                    pos += 1;
                }
                let end = chars.get(pos).map(|&(at, _)| at).unwrap_or(body.len());
                found.push((index, &body[index..end]));
                debug_assert!(start < pos);
            }
            _ => pos += 1,
        }
    }

    Some(found)
}

/// Every identifier equal to `name`, as byte offsets.
pub fn occurrences(body: &str, name: &str) -> Option<Vec<usize>> {
    Some(
        idents(body)?
            .into_iter()
            .filter(|(_, text)| *text == name)
            .map(|(offset, _)| offset)
            .collect(),
    )
}

fn is_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(body: &str) -> Vec<&str> {
        idents(body)
            .expect("scannable")
            .into_iter()
            .map(|(_, name)| name)
            .collect()
    }

    #[test]
    fn code_yields_its_identifiers_in_order() {
        assert_eq!(names("let plan = apply(x);"), vec!["let", "plan", "apply", "x"]);
    }

    #[test]
    fn a_name_inside_a_string_is_not_an_identifier() {
        assert_eq!(names(r#"println!("apply {}", n)"#), vec!["println", "n"]);
    }

    #[test]
    fn a_name_inside_a_raw_string_is_not_an_identifier() {
        assert_eq!(names(r##"let s = r#"apply()"#;"##), vec!["let", "s"]);
    }

    #[test]
    fn a_name_inside_a_comment_is_not_an_identifier() {
        assert_eq!(names("apply(); // apply is called\nplan;"), vec!["apply", "plan"]);
        assert_eq!(names("/* apply */ plan"), vec!["plan"]);
    }

    #[test]
    fn a_lifetime_is_not_an_identifier() {
        assert_eq!(names("fn f<'apply>(x: &'apply u8)"), vec!["fn", "f", "x", "u8"]);
    }

    #[test]
    fn a_char_literal_is_not_an_identifier() {
        assert_eq!(names("let c = 'a'; let d = b'a';"), vec!["let", "c", "let", "d"]);
    }

    #[test]
    fn offsets_point_at_the_identifier() {
        let body = "go(apply);";
        let found = idents(body).expect("scannable");

        assert_eq!(&body[found[1].0..found[1].0 + found[1].1.len()], "apply");
    }

    #[test]
    fn an_unterminated_string_is_refused() {
        assert_eq!(idents("let s = \"abc;"), None);
        assert_eq!(idents("let s = \"a\"; /* open"), None);
    }

    #[test]
    fn occurrences_finds_only_whole_identifiers() {
        assert_eq!(occurrences("apply; applier; apply", "apply"), Some(vec![0, 16]));
    }
}
