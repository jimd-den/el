use super::id::{ElId, Alias};
use super::kind::Kind;

#[derive(Debug, Clone)]
pub struct Element {
    pub id: ElId,
    pub kind: Kind,
    pub name: Option<String>,
    pub alias: Option<Alias>,
    pub parent: Option<ElId>,
    pub body: Option<String>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility { Pub, Crate, Private }

impl Element {
    pub fn new(id: ElId, kind: Kind) -> Self {
        Self { id, kind, name: None, alias: None, parent: None, body: None, visibility: Visibility::Private }
    }
}
