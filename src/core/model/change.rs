// The vocabulary of mutation. A change says what the user asked for, in the
// model's words, and nothing about how it will be carried out: no offsets, no
// files, no ordering. That is what lets `plan` be a pure function — the same
// change against the same catalog always means the same thing (FR-18).
//
// Ownership is explicit: every field is owned (`String`, not `&str`), because a
// change outlives the command line that produced it and gets recorded in history
// (M8). A borrowed form would tie a record's lifetime to the argv it came from.
//
// It deliberately does not carry an id. Ids are issued by the catalog's counter,
// never chosen by a caller, or "never reused" (FR-1) would be a promise the
// caller could break.

use super::body::Body;
use super::element::{BodyMode, Visibility};
use super::id::ElId;
use super::kind::Kind;

/// Where a statement is inserted or moved to, in its owner's body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    End,
    After(ElId),
    Before(ElId),
    /// Zero-based position among the siblings.
    At(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A new item in `module`, its head written keyword-first.
    AddElement {
        kind: Kind,
        module: String,
        signature: String,
        visibility: Visibility,
        /// The body, already parsed by `text::grammar`; `el` supplies the
        /// closing braces and semicolons, so this is a tree, not raw text.
        body: Body,
        /// `split` for a parsed body, `opaque` when policy says never split or
        /// the grammar could not prove a boundary (FR-63, FR-72).
        mode: BodyMode,
    },
    /// Replace any of an item's head, visibility and body; `None` keeps that
    /// field byte for byte (FR-7). A rename is a separate change because it
    /// rewrites references.
    EditElement {
        id: ElId,
        head: Option<String>,
        visibility: Option<Visibility>,
        body: Option<Body>,
        mode: Option<BodyMode>,
    },
    /// Move an item to another module, keeping its id and its body.
    MoveElement { id: ElId, module: String },
    /// Change an item's name. References are rewritten by `plan::refs`.
    Rename { id: ElId, name: String },
    /// Insert one parsed statement (and its children) at an anchor.
    AddStmt { owner: ElId, body: Body, anchor: Anchor },
    /// Replace one statement's text and children, keeping its id.
    EditStmt { id: ElId, body: Body },
    /// Replace a body without saying whether the id is an element or a statement;
    /// the apply layer resolves it against the catalog.
    EditBody { id: ElId, body: Body, mode: Option<BodyMode> },
    /// Reorder a statement after another in the same body.
    ReorderStmt { id: ElId, after: ElId },
    /// Re-parent a statement. `to` is an element or a statement; a target in
    /// another body is refused (`model.cross_body_move`).
    MoveStmt { id: ElId, to: ElId },
    /// Remove an element (with its body) or a statement (with its children).
    Remove { id: ElId },
    /// Add a dependency, which `el.policy` may deny (`model.dep_denied`).
    AddDep { name: String },
}
