// An element is one addressable piece of a Rust program: its permanent id, what
// kind of thing it is, where it lives, what it is called, and how its body is
// stored.
//
// The head is text, not a typed signature (`signature::split` says where it
// starts). That is ADR-0008's stance and the reason rendering can reproduce
// `fn apply(p: &Plan) -> Verdict` without `el` ever parsing Rust: the model
// remembers the characters, the compiler judges them.
//
// The alias is never passed in; it is recomputed whenever the name or module
// changes, so a rename cannot leave a stale alias behind (FR-2). The body itself
// is not here: bodies are statements (`stmt`), and `mode` says whether the split
// is trusted or the body is one opaque blob.

use super::id::{Alias, ElId};
use super::kind::Kind;
use super::signature;

// Defined in `modifier` because they describe presentation, not identity; the
// path stays `element::…` because that is what an element is made of.
pub use super::modifier::{BodyMode, Visibility};

/// What a caller knows when an element is created. `signature` is the head
/// exactly as written on the command line, keyword first: `fn apply(p: &Plan)`,
/// `struct Point`, `impl Display for Point`.
#[derive(Debug, Clone, Copy)]
pub struct NewElement<'a> {
    pub kind: Kind,
    pub module: &'a str,
    pub signature: &'a str,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub id: ElId,
    pub kind: Kind,
    pub name: Option<String>,
    /// Text after the identifier, up to but not including the body: `(x: u8)`
    /// for a function, `: usize = 10` for a const, empty for a struct.
    pub head: String,
    pub alias: Option<Alias>,
    pub module: String,
    pub parent: Option<ElId>,
    pub visibility: Visibility,
    pub mode: BodyMode,
}

impl Element {
    /// Build an element for an id the catalog has already issued.
    ///
    /// `None` when the signature does not begin with the kind's keyword, or a
    /// named kind has no identifier after it — both are L0's to report (FR-5);
    /// this only refuses to invent one.
    pub fn create(id: ElId, new: &NewElement<'_>) -> Option<Self> {
        let (name, head) = signature::split(new.kind, new.signature)?;
        let mut element = Self {
            id,
            kind: new.kind,
            name: name.map(str::to_string),
            head: head.to_string(),
            alias: None,
            module: new.module.to_string(),
            parent: None,
            visibility: new.visibility,
            mode: BodyMode::Split,
        };
        element.realias();
        Some(element)
    }

    /// Changing the name keeps the id and changes the alias (FR-1); the head is
    /// untouched, which is why a rename cannot mangle a parameter list.
    pub fn rename(&mut self, name: &str) {
        self.name = Some(name.to_string());
        self.realias();
    }

    /// Changing the module keeps the id and changes the alias (FR-1).
    pub fn move_to(&mut self, module: &str) {
        self.module = module.to_string();
        self.realias();
    }

    /// Recompute the alias from the current name, kind and module. Called after
    /// decode, which assigns fields directly rather than going through `create`.
    pub(crate) fn realias(&mut self) {
        self.alias = match (&self.name, self.kind.needs_name()) {
            (Some(name), true) => Some(Alias::derive(&self.module, self.kind, name)),
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPLY: NewElement<'_> = NewElement {
        kind: Kind::Function,
        module: "src/core/plan.rs",
        signature: "fn apply(p: &Plan) -> Verdict",
        visibility: Visibility::Pub,
    };

    fn element(new: &NewElement<'_>) -> Element {
        Element::create(ElId(7), new).expect("valid signature")
    }

    #[test]
    fn an_element_derives_its_alias() {
        assert_eq!(element(&APPLY).alias.unwrap().as_str(), "core::plan::fn/apply");
    }

    #[test]
    fn a_rename_keeps_the_id_and_moves_the_alias() {
        let mut element = element(&APPLY);

        element.rename("dispatch");

        assert_eq!(element.id, ElId(7));
        assert_eq!(element.alias.unwrap().as_str(), "core::plan::fn/dispatch");
    }

    #[test]
    fn a_rename_keeps_the_head() {
        let mut element = element(&APPLY);

        element.rename("dispatch");

        assert_eq!(element.head, "(p: &Plan) -> Verdict");
    }

    #[test]
    fn a_move_keeps_the_id_and_moves_the_alias() {
        let mut element = element(&APPLY);

        element.move_to("src/core/gate.rs");

        assert_eq!(element.id, ElId(7));
        assert_eq!(element.alias.unwrap().as_str(), "core::gate::fn/apply");
    }

    #[test]
    fn a_mismatched_signature_creates_nothing() {
        let wrong = NewElement {
            signature: "struct Point",
            ..APPLY
        };

        assert_eq!(Element::create(ElId(1), &wrong), None);
    }

}
