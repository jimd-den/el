// Scalar tokens: the three literals, and numbers kept exactly as written.
//
// Numbers are validated against the JSON grammar and then stored as text. `el`
// never does arithmetic on a number it reads, and `f64` cannot hold cargo's
// 64-bit ids exactly — rounding one would surface as a wrong span three layers
// away (ADR-0008).
//
// It does not normalise: `1e3` stays `1e3` and `-0` stays `-0`, because the
// writer's job is to reproduce what was read, not to have an opinion about it.

use super::Json;
use super::parse::Parser;

impl Parser<'_> {
    pub(super) fn literal(&mut self, word: &str, value: Json) -> Option<Json> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Some(value)
        } else {
            None
        }
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`, kept as written.
    pub(super) fn number(&mut self) -> Option<Json> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        match self.peek()? {
            b'0' => self.pos += 1,
            b'1'..=b'9' => self.skip_digits(),
            _ => return None,
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !self.peek()?.is_ascii_digit() {
                return None;
            }
            self.skip_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !self.peek()?.is_ascii_digit() {
                return None;
            }
            self.skip_digits();
        }

        let text = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        Some(Json::Number(text.to_string()))
    }

    pub(super) fn skip_digits(&mut self) {
        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse;

    #[test]
    fn literals_parse() {
        assert_eq!(parse("true"), Some(super::Json::Bool(true)));
        assert_eq!(parse("false"), Some(super::Json::Bool(false)));
        assert_eq!(parse("null"), Some(super::Json::Null));
    }

    #[test]
    fn a_number_is_kept_as_written() {
        assert_eq!(parse("1.50"), Some(super::Json::Number("1.50".to_string())));
        assert_eq!(parse("-0"), Some(super::Json::Number("-0".to_string())));
    }

    #[test]
    fn a_bad_number_is_refused() {
        for text in ["01", "+1", ".5", "1.", "NaN", "Infinity", "-"] {
            assert_eq!(parse(text), None, "accepted {text}");
        }
    }

}
