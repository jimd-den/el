// String literals, both directions, because getting them consistent in one place
// is what makes `parse(write(x)) == x` hold for text containing quotes, newlines
// and astral codepoints.
//
// Escape handling is the full JSON set: `\" \\ \/ \b \f \n \r \t` and `\uXXXX`,
// including surrogate pairs, since cargo diagnostics can quote arbitrary source
// text. Codepoints outside the BMP are one character in `el` and two escapes on
// the wire, so the pair is joined here.
//
// Writing is minimal: only what JSON requires is escaped, and control characters
// below 0x20 use `\u00XX` rather than the short forms, because one rule is easier
// to verify than a table.

/// The escaped body of `text`, quotes included.
pub(super) fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');

    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Decode a string literal starting at `start`, returning the text and the
/// position just past the closing quote; `None` if it is not a valid literal.
pub(super) fn decode(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut pos = start + 1;
    let mut out: Vec<u8> = Vec::new();

    loop {
        let byte = *bytes.get(pos)?;
        match byte {
            b'"' => return Some((String::from_utf8(out).ok()?, pos + 1)),
            b'\\' => {
                let escape = *bytes.get(pos + 1)?;
                pos += 2;
                match escape {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'/' => out.push(b'/'),
                    b'b' => out.push(0x08),
                    b'f' => out.push(0x0c),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'u' => {
                        let (c, next) = codepoint(bytes, pos)?;
                        pos = next;
                        let mut buffer = [0u8; 4];
                        out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                    }
                    _ => return None,
                }
            }
            byte if byte < 0x20 => return None,
            _ => {
                out.push(byte);
                pos += 1;
            }
        }
    }
}

/// The codepoint of a `\uXXXX` escape (pos just after `u`), joining surrogate
/// pairs; returns the character and the position after the escape.
fn codepoint(bytes: &[u8], pos: usize) -> Option<(char, usize)> {
    let high = hex4(bytes, pos)?;
    if (0xd800..0xdc00).contains(&high) {
        if bytes.get(pos + 4) != Some(&b'\\') || bytes.get(pos + 5) != Some(&b'u') {
            return None;
        }
        let low = hex4(bytes, pos + 6)?;
        if !(0xdc00..0xe000).contains(&low) {
            return None;
        }
        let joined = 0x10000 + ((high as u32 - 0xd800) << 10) + (low as u32 - 0xdc00);
        return Some((char::from_u32(joined)?, pos + 10));
    }
    Some((char::from_u32(high as u32)?, pos + 4))
}

fn hex4(bytes: &[u8], pos: usize) -> Option<u16> {
    let digits = bytes.get(pos..pos + 4)?;
    let text = std::str::from_utf8(digits).ok()?;
    if !text.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u16::from_str_radix(text, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_round_trips() {
        let text = "a\"b\\c\nd\te\u{1}f";

        let encoded = encode(text);
        let (decoded, end) = decode(encoded.as_bytes(), 0).expect("valid");

        assert_eq!(decoded, text);
        assert_eq!(end, encoded.len());
    }

    #[test]
    fn an_escaped_quote_does_not_end_the_string() {
        assert_eq!(decode(br#""a\"b""#, 0).map(|(text, _)| text), Some("a\"b".to_string()));
    }

    #[test]
    fn a_surrogate_pair_is_one_character() {
        let decoded = decode(br#""\ud83d\ude00""#, 0).expect("valid");

        assert_eq!(decoded.0, "😀");
    }

    #[test]
    fn a_lone_surrogate_is_refused() {
        assert_eq!(decode(br#""\ud83d""#, 0), None);
    }

    #[test]
    fn an_unknown_escape_is_refused() {
        assert_eq!(decode(br#""\q""#, 0), None);
    }

    #[test]
    fn a_raw_control_character_is_refused() {
        assert_eq!(decode(b"\"a\x01b\"", 0), None);
    }

    #[test]
    fn an_unterminated_string_is_refused() {
        assert_eq!(decode(b"\"abc", 0), None);
    }
}
