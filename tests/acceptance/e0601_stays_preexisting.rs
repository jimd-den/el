// E0601 names the crate, not an element: adding an item to a main-less
// crate must not be rejected just because the "no main" error's span now
// sits on the new item. The scaffold's E0601 stays pre-existing until
// `fn main` lands, and landing it fixes the error.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ROOT: &str = "target/acceptance/e0601-stays";
const BIN: &str = env!("CARGO_BIN_EXE_el");

fn fresh_root(test: &str) -> PathBuf {
    let root = Path::new(ROOT).join(test);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("create root");
    root
}

fn el(cwd: &Path, args: &[&str]) -> Output {
    Command::new(BIN).current_dir(cwd).args(args).output().expect("run el")
}

fn el_stdin(cwd: &Path, args: &[&str], body: &str) -> Output {
    let mut child = Command::new(BIN)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn el");
    {
        use std::io::Write;
        child.stdin.as_mut().expect("stdin").write_all(body.as_bytes()).expect("write body");
    }
    child.wait_with_output().expect("wait")
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn adding_a_struct_to_a_main_less_crate_is_not_rejected() {
    let root = fresh_root("struct-first");
    let out = el(&root, &["new", "probe"]);
    ok(&out, "new");
    let probe = root.join("probe");

    let out = el_stdin(
        &probe,
        &["add", "struct", "Task", "--vis", "pub", "--sig", "struct Task", "--body", "-"],
        "pub id: u32,\npub done: bool,\n",
    );
    ok(&out, "add struct to a main-less crate");

    let out = el_stdin(
        &probe,
        &["add", "fn", "main", "--sig", "fn main()", "--body", "-"],
        "println!(\"probe\");\n",
    );
    ok(&out, "add main fixes E0601");
}
