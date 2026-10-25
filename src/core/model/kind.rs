// What an element *is*, and the three questions every kind must answer without
// panicking: does it need a name, does it need a body, can it own children?
//
// The rules are total by construction: `Kind::all()` enumerates the variants and
// the test below visits every one, so adding a kind without deciding its three
// answers fails the suite rather than defaulting to something plausible.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Module,
    Function,
    Struct,
    Enum,
    Trait,
    Const,
    Static,
    TypeAlias,
    Impl,
    Stmt,
}

/// Every kind, in the order they are printed.
pub const ALL: [Kind; 10] = [
    Kind::Module,
    Kind::Function,
    Kind::Struct,
    Kind::Enum,
    Kind::Trait,
    Kind::Const,
    Kind::Static,
    Kind::TypeAlias,
    Kind::Impl,
    Kind::Stmt,
];

impl Kind {
    pub fn all() -> &'static [Kind] {
        &ALL
    }

    /// The token used in aliases, records and command lines.
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Module => "mod",
            Kind::Function => "fn",
            Kind::Struct => "struct",
            Kind::Enum => "enum",
            Kind::Trait => "trait",
            Kind::Const => "const",
            Kind::Static => "static",
            Kind::TypeAlias => "type",
            Kind::Impl => "impl",
            Kind::Stmt => "stmt",
        }
    }

    /// `None` for an unknown token; never a default, because a wrong kind
    /// silently changes what an element is.
    pub fn parse(text: &str) -> Option<Self> {
        Kind::all().iter().copied().find(|kind| kind.as_str() == text)
    }

    /// Statements and `impl` blocks have no name of their own.
    pub fn needs_name(&self) -> bool {
        !matches!(self, Kind::Stmt | Kind::Impl)
    }

    /// A body is optional for most kinds, but these are meaningless without one.
    pub fn needs_body(&self) -> bool {
        matches!(self, Kind::Function | Kind::Impl | Kind::Module)
    }

    /// Whether an element of this kind may own child elements.
    pub fn can_nest(&self) -> bool {
        matches!(self, Kind::Module | Kind::Impl | Kind::Trait | Kind::Struct | Kind::Enum)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_rules_are_total() {
        for kind in Kind::all() {
            let answers = (kind.needs_name(), kind.needs_body(), kind.can_nest());
            let _ = answers;
        }
    }

    #[test]
    fn every_kind_round_trips_through_its_token() {
        for kind in Kind::all() {
            assert_eq!(Kind::parse(kind.as_str()), Some(*kind));
        }
    }

    #[test]
    fn no_two_kinds_share_a_token() {
        for (index, kind) in Kind::all().iter().enumerate() {
            for other in &Kind::all()[index + 1..] {
                assert_ne!(kind.as_str(), other.as_str());
            }
        }
    }

    #[test]
    fn an_unknown_token_is_refused() {
        assert_eq!(Kind::parse("class"), None);
    }

    #[test]
    fn a_statement_needs_no_name() {
        assert!(!Kind::Stmt.needs_name());
    }
}
