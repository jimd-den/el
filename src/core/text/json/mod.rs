// JSON, read and written by hand (ADR-0008), for three callers that all ship
// their data as it: `cargo --message-format=json`, `rust-analyzer`, and our own
// `--json` output. It has to be right in both directions, which is why it lives
// in `core` and is tested without any of those tools present.
//
// The value keeps a number as its source text. `el` never does arithmetic on a
// number it reads, and parsing to `f64` would silently round the `u64` ids in
// cargo's diagnostics — a bug that shows up as a wrong span, three layers away.
//
// The writer emits a canonical form: no spaces, object keys in insertion order,
// minimal escaping. Canonical means two runs that agree on the data produce the
// same bytes, which is what lets `--json` be compared and snapshotted.

mod parse;
mod scalar;
mod string;

pub use parse::parse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    /// The number exactly as written; see the module preamble.
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// Canonical text for `value`.
pub fn write(value: &Json) -> String {
    let mut out = String::new();
    write_into(value, &mut out);
    out
}

fn write_into(value: &Json, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(text) => out.push_str(text),
        Json::Str(text) => out.push_str(&string::encode(text)),
        Json::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Json::Object(fields) => {
            out.push('{');
            for (index, (key, field)) in fields.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&string::encode(key));
                out.push(':');
                write_into(field, out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_writes_canonically() {
        let value = Json::Object(vec![
            ("b".to_string(), Json::Array(vec![Json::Null, Json::Bool(true)])),
            ("a".to_string(), Json::Number("1".to_string())),
        ]);

        assert_eq!(write(&value), r#"{"b":[null,true],"a":1}"#);
    }

    #[test]
    fn a_string_is_escaped_minimally() {
        assert_eq!(write(&Json::Str("a\"b\\c\nd".to_string())), r#""a\"b\\c\nd""#);
    }

    #[test]
    fn a_control_character_is_escaped_as_a_codepoint() {
        assert_eq!(write(&Json::Str("a\u{1}b".to_string())), r#""a\u0001b""#);
    }

    #[test]
    fn a_number_keeps_its_text() {
        assert_eq!(write(&Json::Number("18446744073709551615".to_string())), "18446744073709551615");
    }

    #[test]
    fn a_field_can_be_looked_up() {
        let value = Json::Object(vec![("name".to_string(), Json::Str("el".to_string()))]);

        assert_eq!(value.get("name").and_then(Json::as_str), Some("el"));
        assert_eq!(value.get("missing"), None);
    }

    #[test]
    fn a_document_survives_parse_and_write() {
        let text = r#"{"a":[1,{"b":null},true,"x\ny"],"c":-1.5e-3}"#;

        let value = parse(text).expect("valid");
        let written = write(&value);
        let again = parse(&written).expect("canonical output is valid");

        assert_eq!(again, value);
        assert_eq!(write(&again), written);
    }
}
