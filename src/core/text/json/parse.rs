// The parser for `text::json`, and the one place that decides what counts as
// malformed. It is strict on purpose: cargo and rust-analyzer emit valid JSON, so
// anything else in their output is a signal that the stream was cut or the flag
// is wrong, and a lenient reader would hide that behind a plausible value.
//
// Rejected loudly, because JSON says so and because a silently accepted document
// is worse than a refused one: trailing commas, unquoted keys, single quotes,
// leading zeros, `+1`, `.5`, `NaN`, control characters inside strings, an
// unterminated string, and any trailing text after the document.
//
// It does not stream: these documents arrive whole, and a cursor over bytes is
// easier to prove correct than a state machine that can be resumed.

use super::string;
use super::Json;

/// Parse one complete document; `None` for anything not strictly valid.
pub fn parse(text: &str) -> Option<Json> {
    let mut parser = Parser { bytes: text.as_bytes(), pos: 0 };
    parser.skip_whitespace();
    let value = parser.value()?;
    parser.skip_whitespace();

    if parser.pos == parser.bytes.len() { Some(value) } else { None }
}

pub(super) struct Parser<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) pos: usize,
}

impl Parser<'_> {
    pub(super) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(super) fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    pub(super) fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Option<Json> {
        match self.peek()? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => {
                let (text, next) = string::decode(self.bytes, self.pos)?;
                self.pos = next;
                Some(Json::Str(text))
            }
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn object(&mut self) -> Option<Json> {
        self.pos += 1;
        let mut fields = Vec::new();
        self.skip_whitespace();
        if self.peek()? == b'}' {
            self.pos += 1;
            return Some(Json::Object(fields));
        }

        loop {
            self.skip_whitespace();
            if self.peek()? != b'"' {
                return None;
            }
            let (key, next) = string::decode(self.bytes, self.pos)?;
            self.pos = next;
            self.skip_whitespace();
            if self.bump()? != b':' {
                return None;
            }
            self.skip_whitespace();
            fields.push((key, self.value()?));
            self.skip_whitespace();
            match self.bump()? {
                b',' => continue,
                b'}' => return Some(Json::Object(fields)),
                _ => return None,
            }
        }
    }

    fn array(&mut self) -> Option<Json> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek()? == b']' {
            self.pos += 1;
            return Some(Json::Array(items));
        }

        loop {
            self.skip_whitespace();
            items.push(self.value()?);
            self.skip_whitespace();
            match self.bump()? {
                b',' => continue,
                b']' => return Some(Json::Array(items)),
                _ => return None,
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scalar_parses() {
        assert_eq!(parse("null"), Some(Json::Null));
        assert_eq!(parse(" true "), Some(Json::Bool(true)));
        assert_eq!(parse("\"x\""), Some(Json::Str("x".to_string())));
        assert_eq!(parse("-1.5e-3"), Some(Json::Number("-1.5e-3".to_string())));
    }

    #[test]
    fn a_nested_document_parses() {
        let json = parse(r#"{"a":[1,{"b":null}]}"#).expect("valid");

        assert_eq!(
            json.get("a").and_then(Json::as_array).map(|items| items.len()),
            Some(2)
        );
    }

    #[test]
    fn a_trailing_comma_is_refused() {
        assert_eq!(parse("[1,]"), None);
        assert_eq!(parse(r#"{"a":1,}"#), None);
    }

    #[test]
    fn an_unquoted_key_is_refused() {
        assert_eq!(parse("{a:1}"), None);
    }

    #[test]
    fn single_quotes_are_refused() {
        assert_eq!(parse("'a'"), None);
    }

    #[test]
    fn trailing_text_is_refused() {
        assert_eq!(parse("{} {}"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn a_truncated_document_is_refused() {
        for text in [r#"{"a""#, "[1", "\"abc", "{\"a\":}"] {
            assert_eq!(parse(text), None, "accepted {text}");
        }
    }
}
