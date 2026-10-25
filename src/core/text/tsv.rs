// The catalog's on-disk form is line-oriented records (HISTORY §4.3), which
// means every field has to survive being written on one line. This module is the
// escaping that makes that true: a field may contain anything, and the encoded
// form contains no tab, no newline and no carriage return.
//
// The escape set is deliberately tiny — `\\`, `\t`, `\n`, `\r` — because the
// format is meant to stay greppable. Anything else is a decode error rather than
// a guess: a silently mangled field is how a catalog stops being the source of
// truth.
//
// It deliberately does not quote fields or handle comments: a record is fields,
// one line, nothing else. Reading a human-edited catalog is `doctor`'s problem,
// and it starts by rebuilding rather than by forgiving.

/// One record field, with none of the characters that would break a line.
pub fn escape(field: &str) -> String {
    let mut encoded = String::with_capacity(field.len());

    for c in field.chars() {
        match c {
            '\\' => encoded.push_str("\\\\"),
            '\t' => encoded.push_str("\\t"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            other => encoded.push(other),
        }
    }
    encoded
}

/// The inverse of `escape`; `None` on any escape sequence it does not know.
pub fn unescape(field: &str) -> Option<String> {
    let mut decoded = String::with_capacity(field.len());
    let mut chars = field.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            decoded.push(c);
            continue;
        }
        match chars.next()? {
            '\\' => decoded.push('\\'),
            't' => decoded.push('\t'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            _ => return None,
        }
    }
    Some(decoded)
}

/// Records as text: one line per record, fields separated by tabs.
pub fn encode(records: &[Vec<String>]) -> String {
    let mut text = String::new();

    for record in records {
        let line: Vec<String> = record.iter().map(|field| escape(field)).collect();
        text.push_str(&line.join("\t"));
        text.push('\n');
    }
    text
}

/// Text back to records; `None` on an unknown escape or a blank line, because
/// both mean the file is not what was written.
pub fn decode(text: &str) -> Option<Vec<Vec<String>>> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return None;
    }

    let mut records = Vec::new();
    for line in body.split('\n') {
        if line.is_empty() {
            return None;
        }
        let fields: Option<Vec<String>> = line.split('\t').map(unescape).collect();
        records.push(fields?);
    }
    Some(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[&str]) -> Vec<String> {
        fields.iter().map(|field| field.to_string()).collect()
    }

    #[test]
    fn a_tab_does_not_survive_escaping() {
        let encoded = escape("a\tb");

        assert!(!encoded.contains('\t'));
        assert_eq!(unescape(&encoded).as_deref(), Some("a\tb"));
    }

    #[test]
    fn a_newline_does_not_survive_escaping() {
        let encoded = escape("a\nb\rc");

        assert!(!encoded.contains('\n') && !encoded.contains('\r'));
        assert_eq!(unescape(&encoded).as_deref(), Some("a\nb\rc"));
    }

    #[test]
    fn a_backslash_survives() {
        assert_eq!(unescape(&escape("a\\b")).as_deref(), Some("a\\b"));
    }

    #[test]
    fn an_unknown_escape_is_refused() {
        assert_eq!(unescape("a\\qb"), None);
        assert_eq!(unescape("a\\"), None);
    }

    #[test]
    fn records_round_trip() {
        let records = vec![
            record(&["E", "el-0001", "fn", "core::plan::fn/apply"]),
            record(&["S", "el-0002", "stmt", "-", "a\tb\nc"]),
        ];

        assert_eq!(decode(&encode(&records)).unwrap(), records);
    }

    #[test]
    fn two_records_are_two_lines() {
        let records = vec![record(&["P", "ceiling", "200"]), record(&["P", "deps", "none"])];

        assert_eq!(encode(&records).lines().count(), 2);
    }

    #[test]
    fn a_blank_line_is_refused() {
        assert_eq!(decode("C\t1\n\nE\tel-0001\n"), None);
    }
}
