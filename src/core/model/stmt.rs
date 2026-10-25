#[derive(Debug, Clone)]
pub struct Stmt {
    pub id: u32,
    pub text: String,
    pub parent: Option<u32>,
}

impl Stmt {
    pub fn new(id: u32, text: &str) -> Self {
        Self { id, text: text.to_string(), parent: None }
    }
}
