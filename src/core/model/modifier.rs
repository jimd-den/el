// The two per-element modifiers the renderer reads, and nothing else: whether the
// item is `pub`, and whether its body is trusted as split statements or kept as
// one opaque blob (FR-63).
//
// Both are stored in the catalog as tokens rather than as Rust syntax, because a
// record is tab-separated and a token is what `--vis` accepts; `Visibility::prefix`
// is the one place that knows how a token is printed.
//
// It deliberately holds no policy: who may be `pub`, and when a body must go
// opaque, are L0's questions (`plan::validate`), one layer out.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Visibility {
    Pub,
    Crate,
    Private,
}

impl Visibility {
    /// The token the catalog records and `--vis` accepts. Its printed Rust form
    /// is `prefix()`, which is a different question.
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Pub => "pub",
            Visibility::Crate => "crate",
            Visibility::Private => "priv",
        }
    }

    /// The prefix the renderer writes before the keyword, space included.
    pub fn prefix(&self) -> &'static str {
        match self {
            Visibility::Pub => "pub ",
            Visibility::Crate => "pub(crate) ",
            Visibility::Private => "",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pub" => Some(Visibility::Pub),
            "crate" => Some(Visibility::Crate),
            "priv" => Some(Visibility::Private),
            _ => None,
        }
    }
}

/// How a body is stored: split into statements, or kept as one opaque blob
/// (FR-63). Downgrading to opaque is always allowed; upgrading needs a clean
/// split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyMode {
    Split,
    Opaque,
}

impl BodyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BodyMode::Split => "split",
            BodyMode::Opaque => "opaque",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "split" => Some(BodyMode::Split),
            "opaque" => Some(BodyMode::Opaque),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_visibility_round_trips_through_its_token() {
        for visibility in [Visibility::Pub, Visibility::Crate, Visibility::Private] {
            assert_eq!(Visibility::parse(visibility.as_str()), Some(visibility));
        }
    }

    #[test]
    fn a_visibility_prefix_is_its_rust_form() {
        assert_eq!(Visibility::Pub.prefix(), "pub ");
        assert_eq!(Visibility::Crate.prefix(), "pub(crate) ");
        assert_eq!(Visibility::Private.prefix(), "");
    }

    #[test]
    fn a_body_mode_round_trips_through_its_token() {
        for mode in [BodyMode::Split, BodyMode::Opaque] {
            assert_eq!(BodyMode::parse(mode.as_str()), Some(mode));
        }
    }

    #[test]
    fn an_unknown_token_is_refused() {
        assert_eq!(Visibility::parse("pub(super)"), None);
        assert_eq!(BodyMode::parse("mixed"), None);
    }
}
