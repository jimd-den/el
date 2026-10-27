// The cheap references `explain` can see without a language server: a whole-word
// scan over every element head and statement text. It is deliberately not name
// resolution; the page says `source: catalog` for that reason.

use crate::core::model::catalog::Catalog;
use crate::core::model::element::Element;
use crate::core::model::id::ElId;

use super::alias;

pub(super) struct Caller {
    pub(super) id: ElId,
    pub(super) alias: String,
    pub(super) text: String,
}

pub(super) fn callers(catalog: &Catalog, element: &Element) -> Vec<Caller> {
    let Some(name) = &element.name else {
        return Vec::new();
    };
    let mut found: Vec<Caller> = Vec::new();
    for other in &catalog.elements {
        if other.id == element.id || found.iter().any(|caller| caller.id == other.id) {
            continue;
        }
        let hit = catalog
            .statements
            .iter()
            .filter(|stmt| stmt.owner == other.id)
            .find(|stmt| mentions_word(&stmt.text, name))
            .map(|stmt| first_line(&stmt.text))
            .or_else(|| mentions_word(&other.head, name).then(|| first_line(&other.head)));
        if let Some(text) = hit {
            found.push(Caller {
                id: other.id,
                alias: alias(other),
                text,
            });
        }
    }
    found
}

pub(super) fn tests(catalog: &Catalog, element: &Element) -> Vec<Caller> {
    let Some(name) = &element.name else {
        return Vec::new();
    };
    catalog
        .elements
        .iter()
        .filter(|other| other.id != element.id && is_test(other))
        .filter(|other| {
            catalog
                .statements
                .iter()
                .filter(|stmt| stmt.owner == other.id)
                .any(|stmt| mentions_word(&stmt.text, name))
        })
        .map(|other| Caller {
            id: other.id,
            alias: alias(other),
            text: String::new(),
        })
        .collect()
}

fn is_test(element: &Element) -> bool {
    let named = element
        .name
        .as_deref()
        .is_some_and(|name| name == "test" || name.starts_with("test_"));
    named || element.module.contains("tests/")
}

fn mentions_word(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn first_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let clipped: String = line.chars().take(100).collect();
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_matches_only_at_identifier_boundaries() {
        assert!(mentions_word("greet::greet(\"x\")", "greet"));
        assert!(!mentions_word("greeting()", "greet"));
        assert!(!mentions_word("_greet()", "greet"));
        assert!(!mentions_word("greet_2()", "greet"));
    }

    #[test]
    fn a_test_is_named_test_or_test_something() {
        let mut element = Element::create(
            ElId(1),
            &crate::core::model::element::NewElement {
                kind: crate::core::model::kind::Kind::Function,
                module: "src/lib.rs",
                signature: "fn test_greet()",
                visibility: crate::core::model::element::Visibility::Private,
            },
        )
        .expect("element");

        assert!(is_test(&element));
        element.rename("testing");
        assert!(!is_test(&element));
        element.rename("test");
        assert!(is_test(&element));
    }
}
