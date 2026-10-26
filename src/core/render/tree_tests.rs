    use super::*;
    use crate::core::model::element::{NewElement, Visibility};

    fn tree(paths: &[&str]) -> Tree {
        let mut catalog = Catalog::new();
        for path in paths {
            catalog
                .add_element(&NewElement {
                    kind: Kind::Function,
                    module: path,
                    signature: "fn f",
                    visibility: Visibility::Private,
                })
                .expect("add");
        }
        Tree::of(&catalog)
    }

    #[test]
    fn a_path_becomes_a_module_name() {
        assert_eq!(module_of("src/core/plan.rs"), "core::plan");
        assert_eq!(module_of("src/main.rs"), "");
        assert_eq!(module_of("src/lib.rs"), "");
        assert_eq!(module_of("src/a/mod.rs"), "a");
    }

    #[test]
    fn a_grandchild_brings_its_parent_into_being() {
        let tree = tree(&["src/core/plan.rs"]);

        let paths: Vec<&str> = tree.modules().map(|node| node.path.as_str()).collect();

        assert_eq!(paths, vec!["src/core.rs", "src/core/plan.rs", "src/main.rs"]);
    }

    #[test]
    fn a_parent_declares_its_children_once_each() {
        let tree = tree(&["src/a.rs", "src/b.rs"]);

        assert_eq!(tree.get("src/main.rs").expect("root").children, vec!["a", "b"]);
    }

    #[test]
    fn the_root_declares_top_level_modules() {
        let tree = tree(&["src/a.rs"]);

        assert_eq!(tree.root(), "src/main.rs");
        assert!(tree.get("src/main.rs").expect("root").items.is_empty());
    }

    #[test]
    fn a_nested_child_is_declared_by_its_own_parent() {
        let tree = tree(&["src/core/plan.rs", "src/core/gate.rs"]);

        assert_eq!(tree.get("src/core.rs").expect("core").children, vec!["gate", "plan"]);
    }

    #[test]
    fn a_lib_only_catalog_keeps_lib_as_its_root() {
        let tree = tree(&["src/lib.rs"]);

        assert_eq!(tree.root(), "src/lib.rs");
    }

    #[test]
    fn an_empty_catalog_renders_nothing() {
        assert_eq!(tree(&[]).modules().count(), 0);
    }

    #[test]
    fn items_are_in_id_order() {
        let mut catalog = Catalog::new();
        for signature in ["fn b", "fn a"] {
            catalog
                .add_element(&NewElement {
                    kind: Kind::Function,
                    module: "src/main.rs",
                    signature,
                    visibility: Visibility::Private,
                })
                .expect("add");
        }
        // Insertion order and id order are the same above, so make them disagree:
        // removing the sort in `of` must break this test, not just the property.
        catalog.elements.reverse();
        let tree = Tree::of(&catalog);

        assert_eq!(tree.get("src/main.rs").expect("root").items, vec![ElId(1), ElId(2)]);
    }

    #[test]
    fn a_module_element_is_not_an_item() {
        let mut catalog = Catalog::new();
        catalog
            .add_element(&NewElement {
                kind: Kind::Module,
                module: "src/main.rs",
                signature: "mod main",
                visibility: Visibility::Private,
            })
            .expect("add");

        assert!(Tree::of(&catalog).get("src/main.rs").expect("root").items.is_empty());
    }
