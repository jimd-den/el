// `Cargo.toml`, generated rather than hand-maintained, because the manifest is
// part of the projection (ADR-0002): a crate whose source comes from the catalog
// but whose manifest is edited by hand has two sources of truth.
//
// FR-16 is the requirement that matters here: **no dependency table unless the
// catalog declares one**. A generated manifest that grows an empty
// `[dependencies]` invites a human to fill it in, and then the next render
// silently deletes their edit — so absence, not emptiness, is the honest form.
//
// It writes no `[[bin]]` or `[lib]` section either: cargo infers those from
// `src/main.rs` and `src/lib.rs`, which the renderer already decides (FR-15).

/// The manifest for crate `name`, with no dependencies.
///
/// Hazard: `name` is written verbatim into TOML. An invalid package name is an
/// L0 violation (FR-5), and escaping here would hide it rather than report it.
pub fn render(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_names_the_crate() {
        assert!(render("demo").starts_with("[package]\nname = \"demo\"\n"));
    }

    #[test]
    fn a_manifest_declares_no_dependencies() {
        assert!(!render("demo").contains("[dependencies]"));
    }

    #[test]
    fn a_manifest_pins_the_edition() {
        assert!(render("demo").contains("edition = \"2024\""));
    }

    #[test]
    fn two_renders_are_identical() {
        assert_eq!(render("demo"), render("demo"));
    }
}
