// An element is one addressable piece of a Rust program: its permanent id, what
// kind of thing it is, where it lives, what it is called, and how its body is
// stored.
//
// The alias is never passed in; it is recomputed whenever the name or module
// changes, so a rename cannot leave a stale alias behind (FR-2). The body itself
// is not here: bodies live in statements (`stmt`), and `mode` says whether that
// is one opaque blob or a list.

use super::id::{Alias, ElId};
use super::kind::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Visibility {
    Pub,
    Crate,
    Private,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Pub => "pub",
            Visibility::Crate => "crate",
            Visibility::Private => "priv",
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub id: ElId,
    pub kind: Kind,
    pub name: Option<String>,
    pub alias: Option<Alias>,
    pub module: String,
    pub parent: Option<ElId>,
    pub visibility: Visibility,
    pub mode: BodyMode,
}

impl Element {
    /// Build an element for an id the catalog has already issued.
    pub fn new(
        id: ElId,
        kind: Kind,
        module: &str,
        name: Option<&str>,
        visibility: Visibility,
    ) -> Self {
        let mut element = Self {
            id,
            kind,
            name: name.map(str::to_string),
            alias: None,
            module: module.to_string(),
            parent: None,
            visibility,
            mode: BodyMode::Split,
        };
        element.realias();
        element
    }

    /// Changing the name keeps the id and changes the alias (FR-1).
    pub fn rename(&mut self, name: &str) {
        self.name = Some(name.to_string());
        self.realias();
    }

    /// Changing the module keeps the id and changes the alias (FR-1).
    pub fn move_to(&mut self, module: &str) {
        self.module = module.to_string();
        self.realias();
    }

    fn realias(&mut self) {
        self.alias = match (&self.name, self.kind.needs_name()) {
            (Some(name), true) => Some(Alias::derive(&self.module, self.kind, name)),
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element() -> Element {
        Element::new(
            ElId(7),
            Kind::Function,
            "src/core/plan.rs",
            Some("apply"),
            Visibility::Pub,
        )
    }

    #[test]
    fn an_element_derives_its_alias() {
        assert_eq!(element().alias.unwrap().as_str(), "core::plan::fn/apply");
    }

    #[test]
    fn a_rename_keeps_the_id_and_moves_the_alias() {
        let mut element = element();

        element.rename("dispatch");

        assert_eq!(element.id, ElId(7));
        assert_eq!(element.alias.unwrap().as_str(), "core::plan::fn/dispatch");
    }

    #[test]
    fn a_move_keeps_the_id_and_moves_the_alias() {
        let mut element = element();

        element.move_to("src/core/gate.rs");

        assert_eq!(element.id, ElId(7));
        assert_eq!(element.alias.unwrap().as_str(), "core::gate::fn/apply");
    }

    #[test]
    fn a_kind_without_a_name_has_no_alias() {
        let stmt = Element::new(ElId(2), Kind::Stmt, "src/main.rs", None, Visibility::Private);

        assert_eq!(stmt.alias, None);
    }

    #[test]
    fn visibility_and_mode_round_trip() {
        for visibility in [Visibility::Pub, Visibility::Crate, Visibility::Private] {
            assert_eq!(Visibility::parse(visibility.as_str()), Some(visibility));
        }
        for mode in [BodyMode::Split, BodyMode::Opaque] {
            assert_eq!(BodyMode::parse(mode.as_str()), Some(mode));
        }
    }
}
