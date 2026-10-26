// One compiler diagnostic, attributed to the element that owns it — or, when
// no span map names an element, to the file and line it came from. That
// distinction is the fingerprint: an element id survives every edit above it,
// a line number does not, which is why line numbers are deliberately absent
// from the exact fingerprint (GATE.md §4.1).

use crate::core::model::id::ElId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub level: Level,
    /// Innermost element owning the span, from the span map.
    pub el: Option<ElId>,
    /// Owning item, when `el` is a statement.
    pub owner: Option<ElId>,
    pub path: String,
    pub line: u32,
    pub col: u32,
    /// Normalized at parse time: whitespace collapsed, absolute paths erased.
    pub message: String,
    /// False for the opaque fallback, where the span map names no element.
    pub exact: bool,
}

impl Diagnostic {
    /// What identifies this diagnostic across edits. An exact diagnostic is
    /// its code, its element, and its normalized message; a fallback is its
    /// code, its file, its line, and its message — the line is the identity
    /// there because nothing better exists.
    pub fn fingerprint(&self) -> String {
        if self.exact {
            format!(
                "{}|{}|{}",
                self.code,
                self.el.map(|id| id.to_string()).unwrap_or_default(),
                self.message
            )
        } else {
            // The line is display, not identity: a structural edit above a
            // crate-level diagnostic moves its span line, and treating that
            // move as a new error would refuse the very next change (the
            // scaffold's E0601, re-pointed by the first `add mod`, is the
            // witness). Code, file, and message are the stable identity.
            format!("{}|{}|{}", self.code, self.path, self.message)
        }
    }
}

/// Erase what varies without the diagnostic changing: runs of whitespace
/// become one space, and absolute-path tokens become `<path>`. Relative
/// paths stay — they are stable across machines — and type parameter names
/// stay, because they do not vary by position; over-erasing them would merge
/// distinct errors.
pub fn normalize(message: &str) -> String {
    let collapsed = message
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    collapsed
        .split(' ')
        .map(|token| {
            let trimmed = token.trim_matches(|c| c == '`' || c == '"' || c == '\'');
            if trimmed.starts_with('/') {
                "<path>"
            } else {
                token
            }
        })
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod diagnostics_tests;
