use super::Tree;
use super::tree_paths::*;
use crate::core::model::catalog::Catalog;
use crate::core::model::element::Visibility;

impl Tree {
    /// Fold each file's widest item visibility up through every module above it,
    /// so a `mod` line is never narrower than what it declares.
    pub(crate) fn inherit_visibility(&mut self, catalog: &Catalog) {
        let own: Vec<(String, Visibility)> = self
            .nodes
            .values()
            .map(|node| {
                let widest = node
                    .items
                    .iter()
                    .filter_map(|id| catalog.find(*id))
                    .map(|element| element.visibility)
                    .reduce(Visibility::wider)
                    .unwrap_or(Visibility::Private);
                (node.path.clone(), widest)
            })
            .collect();

        for (path, visibility) in own {
            let mut module = module_of(&path);
            loop {
                let file = match module.is_empty() {
                    true => self.root.clone(),
                    false => file_for(&module),
                };
                if let Some(node) = self.nodes.get_mut(&file) {
                    node.visibility = node.visibility.wider(visibility);
                }
                if module.is_empty() {
                    break;
                }
                module = parent_of(&module);
            }
        }
    }
}
