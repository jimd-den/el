    use super::*;
    use crate::core::model::element::{NewElement, Visibility};
    use crate::core::model::kind::Kind;

    fn catalog_with(body: &[&str], mode: BodyMode) -> (Catalog, ElId) {
        let mut catalog = Catalog::new();
        let id = catalog
            .add_element(&NewElement {
                kind: Kind::Function,
                module: "src/a.rs",
                signature: "fn run",
                visibility: Visibility::Private,
            })
            .expect("add");
        for text in body {
            catalog.add_stmt(id, text).expect("stmt");
        }
        if let Some(element) = catalog.find_mut(id) {
            element.mode = mode;
        }
        (catalog, id)
    }

    fn node() -> Node {
        Node {
            path: "src/a.rs".to_string(),
            module: "a".to_string(),
            children: Vec::new(),
            items: Vec::new(),
            visibility: Visibility::Private,
        }
    }

    fn rendered(body: &[&str], mode: BodyMode) -> (String, Vec<super::super::spans::Span>) {
        let (catalog, id) = catalog_with(body, mode);
        let element = catalog.find(id).expect("element").clone();
        let mut out = Out::new();
        let mut spans = Collector::new();
        render(&catalog, &node(), &element, &mut out, &mut spans);

        (out.into_text(), spans.finish())
    }

    #[test]
    fn a_statement_is_indented_one_level() {
        let (text, _) = rendered(&["a();\n"], BodyMode::Split);

        assert_eq!(text, "    a();\n");
    }

    #[test]
    fn stored_indentation_is_recomputed_not_kept() {
        let (text, _) = rendered(&["        a();\n"], BodyMode::Split);

        assert_eq!(text, "    a();\n");
    }

    #[test]
    fn a_blank_line_carries_no_trailing_spaces() {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(&NewElement {
                kind: Kind::Function,
                module: "src/a.rs",
                signature: "fn run",
                visibility: Visibility::Private,
            })
            .expect("add");
        catalog.add_stmt(owner, "a();\n\n").expect("stmt");
        catalog.add_stmt(owner, "b();\n").expect("stmt");
        let element = catalog.find(owner).expect("element").clone();
        let mut out = Out::new();
        let mut spans = Collector::new();

        render(&catalog, &node(), &element, &mut out, &mut spans);

        assert!(!out.text().contains(" \n"), "a line ends in whitespace");
    }

    #[test]
    fn a_child_statement_is_indented_twice() {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(&NewElement {
                kind: Kind::Function,
                module: "src/a.rs",
                signature: "fn run",
                visibility: Visibility::Private,
            })
            .expect("add");
        let outer = catalog.add_stmt(owner, "if x {").expect("stmt");
        catalog.add_child_stmt(outer, "a();").expect("stmt");
        let element = catalog.find(owner).expect("element").clone();
        let mut out = Out::new();
        let mut spans = Collector::new();

        render(&catalog, &node(), &element, &mut out, &mut spans);

        // The closing brace is the renderer's: the model records the children,
        // not the punctuation, and a block nobody closes is not Rust (FR-14).
        assert_eq!(out.into_text(), "    if x {\n        a();\n    }\n");
    }

    #[test]
    fn a_statement_that_opens_no_block_gets_no_brace() {
        let (text, _) = rendered(&["a();\n"], BodyMode::Split);

        assert_eq!(text, "    a();\n");
    }

    #[test]
    fn a_block_inside_a_block_closes_twice() {
        let mut catalog = Catalog::new();
        let owner = catalog
            .add_element(&NewElement {
                kind: Kind::Function,
                module: "src/a.rs",
                signature: "fn run",
                visibility: Visibility::Private,
            })
            .expect("add");
        let outer = catalog.add_stmt(owner, "while x {").expect("stmt");
        let inner = catalog.add_child_stmt(outer, "if y {").expect("stmt");
        catalog.add_child_stmt(inner, "a();").expect("stmt");
        let element = catalog.find(owner).expect("element").clone();
        let mut out = Out::new();
        let mut spans = Collector::new();

        render(&catalog, &node(), &element, &mut out, &mut spans);

        assert_eq!(
            out.into_text(),
            "    while x {\n        if y {\n            a();\n        }\n    }\n"
        );
    }

    #[test]
    fn an_opaque_body_keeps_its_bytes() {
        let (text, _) = rendered(&["if x {\n  a();\n}\n"], BodyMode::Opaque);

        assert_eq!(text, "    if x {\n  a();\n}\n");
    }

    #[test]
    fn a_statement_owns_its_line() {
        let (_, spans) = rendered(&["a();\n"], BodyMode::Split);

        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].first, spans[0].last), (1, 1));
    }
