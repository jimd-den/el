use crate::core::model::catalog::Catalog;

pub fn render(catalog: &Catalog) -> String {
    let mut out = String::new();
    for e in &catalog.elements {
        if let Some(name) = &e.name {
            out.push_str(&format!("// {} {:?}\n", e.id, e.kind));
            out.push_str(&format!("pub fn {}() {{}}\n", name));
        }
    }
    out
}
