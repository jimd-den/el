// Identity is the promise the rest of `el` is built on: an element's id is
// assigned once, printed as `el-NNNN`, and never changes — not on rename, not on
// move, not on undo. This module holds the id and the human alias, and nothing
// else, so that "permanent" is a property of a value rather than a convention.
//
// The alias is derived, never stored by the caller: `<module>::<kind>/<name>`
// (FR-2), which is why the kind is part of it — a `fn apply` and a `const apply`
// in the same module are two elements and must not collide.

use super::kind::Kind;
use std::fmt;

/// A permanent element id. Wraps the counter value; `el-0000` is never issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElId(pub u32);

impl ElId {
    pub fn new(number: u32) -> Self {
        Self(number)
    }

    /// Parse the printed form; `None` for anything that is not `el-NNNN`.
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.strip_prefix("el-")?;
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(Self(digits.parse().ok()?))
    }

    /// The catalog's marker for "no element here", as written in a record.
    pub fn is_unset(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for ElId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "el-{:04}", self.0)
    }
}

/// A unique human name for an element, derived from what it is and where it
/// lives. Not validated against Rust syntax: that is L0's job (`plan::validate`),
/// and an alias is allowed to exist for an element the compiler would reject —
/// the gate is what refuses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias(String);

impl Alias {
    /// `<module>::<kind>/<name>`, where `module` may be a path (`src/a/b.rs`) or
    /// an alias path (`a::b`); both normalise to `a::b`.
    pub fn derive(module: &str, kind: Kind, name: &str) -> Self {
        Self(format!("{}::{}/{}", module_alias(module), kind.as_str(), name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Alias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `src/core/plan.rs` and `core::plan` both become `core::plan`.
fn module_alias(module: &str) -> String {
    let trimmed = module.strip_prefix("src/").unwrap_or(module);
    let trimmed = trimmed.strip_suffix(".rs").unwrap_or(trimmed);
    trimmed.replace('/', "::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_prints_as_el_nnnn() {
        assert_eq!(ElId(11).to_string(), "el-0011");
    }

    #[test]
    fn an_id_parses_from_its_printed_form() {
        assert_eq!(ElId::parse("el-0011"), Some(ElId(11)));
    }

    #[test]
    fn a_malformed_id_is_refused() {
        assert_eq!(ElId::parse("el-"), None);
        assert_eq!(ElId::parse("el-001x"), None);
        assert_eq!(ElId::parse("00011"), None);
    }

    #[test]
    fn an_id_round_trips_at_9999_and_10000() {
        for n in [9999u32, 10000] {
            let id = ElId(n);
            assert_eq!(ElId::parse(&id.to_string()), Some(id));
        }
        assert_eq!(ElId(10000).to_string(), "el-10000");
        assert_eq!(ElId::parse("el-10000"), Some(ElId(10000)));
    }

    #[test]
    fn an_alias_names_module_kind_and_element() {
        let alias = Alias::derive("src/core/plan.rs", Kind::Function, "apply");

        assert_eq!(alias.as_str(), "core::plan::fn/apply");
    }

    #[test]
    fn an_alias_path_module_needs_no_normalising() {
        assert_eq!(
            Alias::derive("core::plan", Kind::Const, "LIMIT").as_str(),
            "core::plan::const/LIMIT"
        );
    }

    #[test]
    fn kind_keeps_same_named_elements_apart() {
        let function = Alias::derive("a", Kind::Function, "x");
        let constant = Alias::derive("a", Kind::Const, "x");

        assert_ne!(function, constant);
    }
}
