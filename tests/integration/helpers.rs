// FR-14: rendered files are valid Rust that `cargo check` accepts when the
// catalog is valid. This is the only test that can make that claim, because
// "valid" is the compiler's word and not ours.
//
// It is deliberately not `#[ignore]`d: a rendering milestone whose compiled
// output is only checked on request is a milestone whose output is not checked.
// The cost is five `cargo check` runs against one shared target directory, which
// is a second or two on a warm machine and the main reason this binary is
// separate from the unit suite.

use el::core::model::catalog::Catalog;
use el::core::model::element::{NewElement, Visibility};
use el::core::model::id::ElId;
use el::core::model::kind::Kind;
use el::core::render::render;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT: &str = "target/integration/rendered";

pub fn new_el<'a>(
    kind: Kind,
    module: &'a str,
    signature: &'a str,
    visibility: Visibility,
) -> NewElement<'a> {
    NewElement {
        kind,
        module,
        signature,
        visibility,
    }
}

pub fn add(catalog: &mut Catalog, kind: Kind, module: &str, signature: &str) -> ElId {
    add_vis(catalog, kind, module, signature, Visibility::Private)
}

pub fn add_vis(
    catalog: &mut Catalog,
    kind: Kind,
    module: &str,
    signature: &str,
    visibility: Visibility,
) -> ElId {
    catalog
        .add_element(&new_el(kind, module, signature, visibility))
        .unwrap_or_else(|error| panic!("add {signature} in {module}: {error:?}"))
}

/// A statement that opens a block, with its contents as children — the model's
/// only way to say "this statement contains those" (FR-62). The renderer supplies
/// the closing brace, because the model records children rather than punctuation.
pub fn block(catalog: &mut Catalog, owner: ElId, opening: &str, lines: &[&str]) -> ElId {
    let id = catalog.add_stmt(owner, opening).expect("stmt");
    for line in lines {
        catalog.add_child_stmt(id, line).expect("child stmt");
    }
    id
}

pub fn stmts(catalog: &mut Catalog, owner: ElId, lines: &[&str]) {
    for line in lines {
        catalog.add_stmt(owner, line).expect("stmt");
    }
}

/// Write every rendered file under `ROOT/<name>` and run `cargo check`.
pub fn assert_compiles(name: &str, catalog: &Catalog) {
    let root = Path::new(ROOT).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    let projection = render(catalog, name);
    for file in &projection.files {
        let path = root.join(&file.path);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, &file.text).expect("write");
    }

    let target = fs::canonicalize("target").expect("target dir").join("integration/target");
    let output = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(["check", "--offline", "--quiet"])
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("run cargo check");

    if !output.status.success() {
        let rendered = fs::read_dir(root.join("src"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| {
                        let text = fs::read_to_string(entry.path()).unwrap_or_default();
                        format!("--- {} ---\n{text}", entry.path().display())
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            })
            .unwrap_or_default();
        panic!(
            "{name} did not compile:\n{}\n{rendered}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// An empty directory under `target/integration` for a test that needs a real
/// store or a real tree on disk.
///
/// Under `target/` so a failing test leaves its evidence where the next run will
/// clean it, and never in the repository. Named by the caller so two tests cannot
/// share one store by accident.
pub fn fresh_dir(name: &str) -> PathBuf {
    let root = Path::new("target/integration").join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("create");
    root
}
