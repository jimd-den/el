// An element's *head* is the text the renderer must reproduce between the
// visibility and the body: `fn apply(p: &Plan) -> Verdict`, `struct Point`,
// `impl Display for Point`. Storing it as text — rather than as a typed
// signature — is ADR-0008's stance: `el` never becomes a Rust parser.
//
// This module is the one place that splits that text into the three things the
// model needs: the keyword (from the kind), the identifier (which the alias is
// derived from, and which a rename must change), and the head proper (which is
// opaque and survives a rename untouched).
//
// It is lexical and deliberately shallow: it finds the first identifier and
// stops. It does not check that the rest is valid Rust — the gate is the
// arbiter (`FR-5`), and a signature this accepts may still fail `cargo check`.

use super::kind::Kind;

/// A head split into its identifier and the text after it.
///
/// Contract: `Some((name, head))` for a named kind always has a non-empty
/// `name`; an unnamed kind (`impl`) has `None` and puts everything after the
/// keyword into `head`, because `impl Display for Point` has no identifier of
/// its own at a fixed position.
pub fn split(kind: Kind, signature: &str) -> Option<(Option<&str>, &str)> {
    let rest = signature.strip_prefix(kind.decl_keyword())?;

    let rest = match rest {
        "" => "",
        _ => rest.strip_prefix(' ')?,
    };

    if !kind.needs_name() {
        // Everything after the keyword's space is the head; the renderer puts the
        // space back, so `impl Display for Point` round-trips.
        return Some((None, rest));
    }
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some((Some(&rest[..end]), &rest[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_function_splits_into_name_and_parameters() {
        assert_eq!(
            split(Kind::Function, "fn apply(p: &Plan) -> Verdict"),
            Some((Some("apply"), "(p: &Plan) -> Verdict"))
        );
    }

    #[test]
    fn a_bare_function_has_an_empty_head() {
        assert_eq!(split(Kind::Function, "fn main"), Some((Some("main"), "")));
    }

    #[test]
    fn generics_stay_on_the_head() {
        assert_eq!(
            split(Kind::Function, "fn first<T>(xs: &[T]) -> Option<&T>"),
            Some((Some("first"), "<T>(xs: &[T]) -> Option<&T>"))
        );
    }

    #[test]
    fn a_const_keeps_its_type_and_value_as_head() {
        assert_eq!(
            split(Kind::Const, "const LIMIT: usize = 10"),
            Some((Some("LIMIT"), ": usize = 10"))
        );
    }

    #[test]
    fn an_impl_has_no_identifier() {
        assert_eq!(
            split(Kind::Impl, "impl Display for Point"),
            Some((None, "Display for Point"))
        );
    }

    #[test]
    fn a_wrong_keyword_is_refused() {
        assert_eq!(split(Kind::Function, "struct Point"), None);
        assert_eq!(split(Kind::Function, "function main"), None);
    }

    #[test]
    fn a_named_kind_with_no_identifier_is_refused() {
        assert_eq!(split(Kind::Function, "fn"), None);
        assert_eq!(split(Kind::Function, "fn (x: u8)"), None);
    }

    #[test]
    fn a_keyword_without_its_space_is_refused() {
        assert_eq!(split(Kind::Function, "fnmain"), None);
    }

    #[test]
    fn a_type_alias_uses_its_own_keyword() {
        assert_eq!(
            split(Kind::TypeAlias, "type Id = u32"),
            Some((Some("Id"), " = u32"))
        );
    }

    #[test]
    fn a_test_splits_with_the_fn_keyword() {
        assert_eq!(
            split(Kind::Test, "fn checks_answer()"),
            Some((Some("checks_answer"), "()"))
        );
    }

    #[test]
    fn a_test_sig_starting_with_test_is_refused() {
        assert_eq!(split(Kind::Test, "test checks_answer()"), None);
    }

    #[test]
    fn a_test_sig_starting_with_an_attribute_is_refused() {
        assert_eq!(split(Kind::Test, "#[test] fn checks_answer()"), None);
    }
}
