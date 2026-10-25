// Reading a body character by character means knowing which characters are
// *not code*: the inside of a string, a raw string, a char literal, a line
// comment, a block comment. This module is that knowledge and nothing else.
// Each function takes a position at the start of a construct and returns the
// position just past it, or `None` when the construct never closes.
//
// It deliberately does not track nesting of delimiters, decide where a
// statement ends, or know anything about Rust grammar: those are the callers'
// jobs, in `terminator`. A `None` here is the earliest and cheapest refusal in
// the chain — an unterminated literal means no boundary in this body is
// provable.

/// The character at `pos`, if the body is long enough.
pub(crate) fn char_at(chars: &[(usize, char)], pos: usize) -> Option<char> {
    chars.get(pos).map(|&(_, c)| c)
}

/// Past the end of a line comment, at the newline (or past the end).
pub(crate) fn skip_line(chars: &[(usize, char)], start: usize) -> usize {
    let mut pos = start;
    while pos < chars.len() && chars[pos].1 != '\n' {
        pos += 1;
    }
    pos
}

/// Past a block comment, which nests in Rust; `None` if it never closes.
pub(crate) fn skip_block_comment(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut pos = start + 2;

    while pos < chars.len() {
        match (char_at(chars, pos), char_at(chars, pos + 1)) {
            (Some('/'), Some('*')) => {
                depth += 1;
                pos += 2;
            }
            (Some('*'), Some('/')) => {
                depth -= 1;
                pos += 2;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => pos += 1,
        }
    }
    None
}

/// Past a `"` string, honouring `\"` escapes; `None` if it never closes.
pub(crate) fn skip_string(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut pos = start + 1;

    while pos < chars.len() {
        match chars[pos].1 {
            '\\' => pos += 2,
            '"' => return Some(pos + 1),
            _ => pos += 1,
        }
    }
    None
}

/// Past a char literal *or* a lifetime/label. The two are told apart lexically:
/// a char literal closes after one character or one escape, so `'a` with no
/// closing quote is a lifetime and is skipped as such. `None` for a stray quote,
/// whose meaning cannot be proved.
///
/// Hazard: this is the lexical reading, not the semantic one. `let c = 'a;` is
/// read as a lifetime followed by `;`, which is what the tokens say.
pub(crate) fn skip_char_or_lifetime(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match char_at(chars, start + 1)? {
        '\\' => match char_at(chars, start + 3) {
            Some('\'') => Some(start + 4),
            _ => None,
        },
        '\'' => None,
        first if char_at(chars, start + 2) == Some('\'') && first != '\n' => Some(start + 3),
        _ => {
            let mut pos = start + 1;
            while char_at(chars, pos).is_some_and(|c| c.is_alphanumeric() || c == '_') {
                pos += 1;
            }
            if pos == start + 1 { None } else { Some(pos) }
        }
    }
}

/// True when `r"…"`, `r#"…"#`, `b"…"` or `br#"…"#` starts here.
///
/// Hazard: `r#type` is a raw identifier, not a raw string. Requiring a quote
/// after the hashes is what keeps the two apart; without it, every raw
/// identifier in a body would be refused.
pub(crate) fn is_raw_string_start(chars: &[(usize, char)], start: usize) -> bool {
    raw_body_start(chars, start).is_some()
}

/// Past a raw string, whose body ends only at `"` followed by its own hash
/// count; `None` if it never closes.
pub(crate) fn skip_raw_string(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let (body, hashes) = raw_body_start(chars, start)?;
    let mut pos = body + 1;

    while pos < chars.len() {
        let closes = chars[pos].1 == '"'
            && (0..hashes).all(|offset| char_at(chars, pos + 1 + offset) == Some('#'));
        if closes {
            return Some(pos + 1 + hashes);
        }
        pos += 1;
    }
    None
}

/// `(index of the opening quote, hash count)` for a raw string at `start`.
fn raw_body_start(chars: &[(usize, char)], start: usize) -> Option<(usize, usize)> {
    let mut pos = start;
    if char_at(chars, pos) == Some('b') {
        pos += 1;
    }
    if char_at(chars, pos) != Some('r') {
        return None;
    }
    pos += 1;

    let mut hashes = 0;
    while char_at(chars, pos) == Some('#') {
        hashes += 1;
        pos += 1;
    }
    if char_at(chars, pos) == Some('"') { Some((pos, hashes)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(body: &str) -> Vec<(usize, char)> {
        body.char_indices().collect()
    }

    #[test]
    fn a_string_is_skipped_whole() {
        assert_eq!(skip_string(&chars("\"a;b\";"), 0), Some(5));
    }

    #[test]
    fn an_escaped_quote_does_not_end_a_string() {
        assert_eq!(skip_string(&chars("\"a\\\";b\";"), 0), Some(7));
    }

    #[test]
    fn block_comments_nest() {
        assert_eq!(skip_block_comment(&chars("/* /* */ ; */ x"), 0), Some(13));
    }

    #[test]
    fn an_unterminated_string_is_none() {
        assert_eq!(skip_string(&chars("\"abc;"), 0), None);
    }

    #[test]
    fn a_lifetime_is_not_a_char_literal() {
        assert_eq!(skip_char_or_lifetime(&chars("'a x"), 0), Some(2));
    }

    #[test]
    fn a_char_literal_closes_immediately() {
        assert_eq!(skip_char_or_lifetime(&chars("';' x"), 0), Some(3));
    }

    #[test]
    fn a_raw_string_honours_its_hashes() {
        assert_eq!(skip_raw_string(&chars("r#\"a\";\"# x"), 0), Some(8));
    }

    #[test]
    fn a_raw_identifier_is_not_a_raw_string() {
        assert!(!is_raw_string_start(&chars("r#type"), 0));
    }
}
