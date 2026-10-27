// `el init` adopts an existing Cargo crate: every file under `src/` becomes one
// opaque module, so the catalog can address, check and edit it at file granularity
// while the bytes on disk stay exactly what the author wrote. The manifest is not
// adopted, because rewriting a user's manifest is not adoption.

use std::fs;
use std::path::Path;

use crate::program::{el, el_stdin, fresh, ok};

fn text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const MAIN: &str = "mod greet;\nfn main() {\n    greet::hello();\n}\n";
const GREET: &str = "pub fn hello() {\n    println!(\"hi\");\n}\n";
const MANIFEST: &str = "[package]\nname = \"adopted\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

fn crate_at(name: &str) -> std::path::PathBuf {
    let root = fresh(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("Cargo.toml"), MANIFEST).expect("manifest");
    fs::write(root.join("src/main.rs"), MAIN).expect("main");
    fs::write(root.join("src/greet.rs"), GREET).expect("greet");
    root
}

fn module_id(root: &Path, alias: &str) -> String {
    let output = el(root, &["find", alias]);
    ok(&output, "find");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.contains(alias))
        .and_then(|line| line.split_whitespace().next())
        .expect("module id")
        .to_string()
}

#[test]
fn init_adopts_the_src_tree_and_leaves_every_byte_alone() {
    let root = crate_at("init-adopt");

    let output = el(&root, &["init"]);

    ok(&output, "init");
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).expect("main"), MAIN);
    assert_eq!(fs::read_to_string(root.join("src/greet.rs")).expect("greet"), GREET);
    assert_eq!(fs::read_to_string(root.join("Cargo.toml")).expect("manifest"), MANIFEST);
    let listing = text(&el(&root, &["ls"]));
    assert!(listing.contains("main::mod/main"), "{listing}");
    assert!(listing.contains("greet::mod/greet"), "{listing}");
    assert!(listing.contains("opaque"), "{listing}");
    let log = text(&el(&root, &["log"]));
    assert!(log.contains("import"), "{log}");
    ok(&el(&root, &["check"]), "check after init");
}

#[test]
fn a_second_init_is_refused() {
    let root = crate_at("init-twice");
    ok(&el(&root, &["init"]), "init");

    let output = el(&root, &["init"]);

    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(text(&output).contains("already"), "{}", text(&output));
}

#[test]
fn el_edits_an_adopted_file_and_undo_restores_it() {
    let root = crate_at("init-edit");
    ok(&el(&root, &["init"]), "init");
    let main = module_id(&root, "main::mod/main");
    let edited = "mod greet;\nfn main() {\n    greet::hello();\n    println!(\"edited\");\n}\n";

    let output = el_stdin(
        &root,
        &["edit", &main, "--body", "-", "--body-mode", "opaque"],
        edited,
    );

    ok(&output, "edit adopted module");
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).expect("main"), edited);
    assert_eq!(fs::read_to_string(root.join("Cargo.toml")).expect("manifest"), MANIFEST);
    let run = text(&el(&root, &["run"]));
    assert!(run.contains("edited"), "{run}");
    ok(&el(&root, &["undo"]), "undo");
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).expect("main"), MAIN);
    assert_eq!(fs::read_to_string(root.join("Cargo.toml")).expect("manifest"), MANIFEST);
}

#[test]
fn an_item_cannot_be_added_to_an_adopted_file() {
    let root = crate_at("init-opaque-add");
    ok(&el(&root, &["init"]), "init");

    let output = el(
        &root,
        &["add", "fn", "extra", "--in", "src/main.rs", "--sig", "fn extra()"],
    );

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(text(&output).contains("opaque"), "{}", text(&output));
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).expect("main"), MAIN);
}
