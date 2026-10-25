use super::element::Element;
use super::id::ElId;

#[derive(Debug, Default)]
pub struct Catalog {
    pub elements: Vec<Element>,
    pub next_id: u32,
}

impl Catalog {
    pub fn new() -> Self { Self { elements: vec![], next_id: 1 } }
    pub fn alloc_id(&mut self) -> ElId {
        let id = ElId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn add(&mut self, mut el: Element) {
        if el.id.0 == 0 { el.id = self.alloc_id(); }
        self.elements.push(el);
    }
    pub fn find(&self, id: ElId) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }
}
