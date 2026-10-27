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

#[test]
fn explain_of_an_adopted_module_is_bounded() {
    let root = crate_at("init-explain");
    ok(&el(&root, &["init"]), "init");
    let main = module_id(&root, "main::mod/main");

    let output = el(&root, &["explain", &main]);

    ok(&output, "explain adopted module");
    let shown = text(&output);
    assert!(shown.len() < 2000, "explain is unbounded: {} bytes", shown.len());
    assert!(!shown.contains("fn main"), "the whole file leaked into explain:\n{shown}");
}

#[cfg(unix)]
#[test]
fn adoption_does_not_follow_a_directory_symlink() {
    use std::os::unix::fs::symlink;

    let root = crate_at("init-symlink");
    let outside = fresh("init-symlink-outside");
    fs::write(outside.join("outside.rs"), "pub fn outside() {}\n").expect("outside");
    let outside = outside.canonicalize().expect("absolute");
    symlink(&outside, root.join("src/linked")).expect("symlink");

    let output = el(&root, &["init"]);

    ok(&output, "init");
    let listing = text(&el(&root, &["ls"]));
    assert!(!listing.contains("outside.rs"), "{listing}");
    assert!(!listing.contains("linked"), "{listing}");
}

#[test]
fn init_refuses_a_layout_it_would_have_to_invent() {
    let root = fresh("init-invent");
    fs::create_dir_all(root.join("src/bin")).expect("src/bin");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"only-bin\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/bin/tool.rs"), "fn main() {}\n").expect("tool");

    let output = el(&root, &["init"]);

    assert_eq!(output.status.code(), Some(4), "{}", text(&output));
    let shown = text(&output);
    assert!(shown.contains("invent"), "{shown}");
    assert!(shown.contains("src/"), "{shown}");
}

#[test]
fn init_keeps_an_existing_gitignore_and_adds_the_store() {
    let root = crate_at("init-gitignore");
    fs::write(root.join(".gitignore"), "target/\n").expect("gitignore");

    ok(&el(&root, &["init"]), "init");

    let text = fs::read_to_string(root.join(".gitignore")).expect("gitignore");
    assert!(text.contains("target/"), "{text}");
    assert!(text.contains(".el/"), "{text}");
}

#[test]
fn init_retries_after_an_unpublished_store() {
    let root = crate_at("init-retry");
    fs::create_dir_all(root.join(".el")).expect("store");
    fs::write(root.join(".el/junk"), "half an adoption").expect("junk");

    ok(&el(&root, &["init"]), "init retry");
}

#[test]
fn an_item_cannot_be_added_under_an_adopted_module() {
    let root = crate_at("init-opaque-ancestor");
    ok(&el(&root, &["init"]), "init");

    let output = el(
        &root,
        &["add", "fn", "deep", "--in", "src/greet/deep.rs", "--sig", "fn deep()"],
    );

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(text(&output).contains("opaque"), "{}", text(&output));
}
