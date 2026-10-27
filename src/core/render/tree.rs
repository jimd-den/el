// The module tree, derived from file paths. FR-15 says users never write `mod`
// declarations, so this is the thing that knows them: which file declares which
// child, and which files must exist because a grandchild does.
//
// The rule is cargo's, minus the parts a projection cannot need: `src/main.rs`
// (or `src/lib.rs`) is the root; `src/a.rs` and `src/a/mod.rs` are both the
// module `a`; `src/a/b.rs` is `a::b` under `a`.
//
// It deliberately does not decide whether a module is _allowed_ to exist — an
// element in a file nobody declares is an L0 violation (FR-18), not a rendering
// question — and it never invents an item, only the empty file a `mod` line
// needs in order for the crate to compile.

use super::super::model::catalog::Catalog;
use super::super::model::element::Visibility;
use super::super::model::id::ElId;
use super::super::model::kind::Kind;
use self::tree_paths::*;
use std::collections::{BTreeMap, BTreeSet};

/// A file in the projection and what belongs in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// `src/core/plan.rs`
    pub path: String,
    /// `core::plan`; empty for the crate root.
    pub module: String,
    /// Child module names, sorted — one `mod <name>;` each.
    pub children: Vec<String>,
    /// Item ids, sorted; the order they are written in.
    pub items: Vec<ElId>,
    /// The widest visibility anything in this file's subtree declares, which is
    /// how visible the `mod` line that declares it has to be.
    ///
    /// Without this the projection hides what the catalog exposed: a `pub fn` in
    /// `src/core/plan.rs` inside a private `mod plan;` cannot be named from
    /// `src/main.rs`, and `cargo check` says so (FR-14).
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// Keyed by path, so a BTreeMap's iteration *is* the canonical file order.
    nodes: BTreeMap<String, Node>,
    root: String,
}

impl Tree {
    pub fn of(catalog: &Catalog) -> Self {
        let mut wanted: BTreeSet<String> = BTreeSet::new();
        for path in mentioned(catalog) {
            for module in ancestors(&module_of(&path)) {
                if !module.is_empty() {
                    wanted.insert(
                        known_file(&module, catalog).unwrap_or_else(|| file_for(&module)),
                    );
                }
            }
            wanted.insert(path);
        }
        if wanted.is_empty() {
            return Self {
                nodes: BTreeMap::new(),
                root: file_for(""),
            };
        }
        let root = if wanted.contains("src/lib.rs") && !wanted.contains("src/main.rs") {
            "src/lib.rs".to_string()
        } else {
            file_for("")
        };
        wanted.insert(root.clone());

        let mut nodes: BTreeMap<String, Node> = wanted
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    Node {
                        path: path.clone(),
                        module: module_of(path),
                        children: Vec::new(),
                        items: Vec::new(),
                        visibility: Visibility::Private,
                    },
                )
            })
            .collect();

        for element in &catalog.elements {
            if element.kind == Kind::Module {
                continue;
            }
            if let Some(node) = nodes.get_mut(&element.module) {
                node.items.push(element.id);
            }
        }

        let links: Vec<(String, String)> = nodes
            .values()
            .filter(|node| !node.module.is_empty())
            .map(|node| (parent_of(&node.module), leaf(&node.module).to_string()))
            .collect();
        for (parent, name) in links {
            let parent = if parent.is_empty() {
                root.clone()
            } else {
                known_file(&parent, catalog).unwrap_or_else(|| file_for(&parent))
            };
            if let Some(node) = nodes.get_mut(&parent) {
                node.children.push(name);
            }
        }

        for node in nodes.values_mut() {
            node.children.sort();
            node.children.dedup();
            node.items.sort();
        }
        let mut tree = Self { nodes, root };
        tree.inherit_visibility(catalog);
        tree
    }

    /// The `mod` lines `path` must carry: each child's name and how visible that
    /// line has to be.
    pub fn mod_lines(&self, path: &str) -> Vec<(String, Visibility)> {
        let Some(node) = self.get(path) else {
            return Vec::new();
        };

        node.children
            .iter()
            .map(|child| {
                let file = child_file(path, child);
                let visibility = self
                    .get(&file)
                    .map(|child| child.visibility)
                    .unwrap_or(Visibility::Private);
                (child.clone(), visibility)
            })
            .collect()
    }

    /// Every node, in path order — the order files are written in.
    pub fn modules(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// The crate root: `src/main.rs`, or `src/lib.rs` when that is the only one.
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn get(&self, path: &str) -> Option<&Node> {
        self.nodes.get(path)
    }
}

/// The file a module already lives in, when the catalog has a module element for
/// it. Cargo accepts `src/a/mod.rs` and `src/a.rs`; the catalog's path wins, so a
/// grandchild does not invent a second file for a module that exists.
fn known_file(module: &str, catalog: &Catalog) -> Option<String> {
    catalog
        .elements
        .iter()
        .find(|element| element.kind == Kind::Module && module_of(&element.module) == module)
        .map(|element| element.module.clone())
}

#[path = "tree_paths.rs"]
mod tree_paths;
#[path = "tree_visibility.rs"]
mod tree_visibility;

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;
