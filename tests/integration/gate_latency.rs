// The gate must stay cheap on a real crate: one gated `add fn` on a
// 30-module program, after two warm runs, finishes inside two seconds on
// this machine. The crate is built with `--fast` adds so the setup does not
// pay the gate sixty times; the warm runs pay it, so the timed run measures
// a single incremental `cargo check`, not a cold cache.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Instant;

const ROOT: &str = "target/integration/gate-latency";
const BIN: &str = env!("CARGO_BIN_EXE_el");

fn fresh_root() -> PathBuf {
    let root = Path::new(ROOT);
    if root.exists() {
        fs::remove_dir_all(root).expect("clean");
    }
    fs::create_dir_all(root).expect("create root");
    root.to_path_buf()
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

fn add_fn(cwd: &Path, module: &str, name: &str, body: &str, fast: bool) {
    let sig = format!("fn {name}() -> u32");
    let mut owned: Vec<String> = vec![
        "add".to_string(),
        "fn".to_string(),
        name.to_string(),
        "--in".to_string(),
        module.to_string(),
        "--sig".to_string(),
        sig,
        "--body".to_string(),
        "-".to_string(),
    ];
    if fast {
        owned.push("--fast".to_string());
    }
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    ok(&el_stdin(cwd, &refs, body), &format!("add fn {name}"));
}

#[test]
fn a_gated_add_on_a_thirty_module_crate_stays_inside_two_seconds() {
    let root = fresh_root();
    let lat = root.join("lat");
    ok(&el(&root, &["new", "lat"]), "el new");
    for index in 0..30 {
        let module = format!("m{index:02}");
        ok(&el(&lat, &["add", "mod", &module, "--fast"]), &format!("add mod {module}"));
    }
    for index in 0..30 {
        let module = format!("m{index:02}");
        let name = format!("f{index:02}");
        add_fn(&lat, &module, &name, "42", true);
    }
    // Two warm gated runs: the first pays the missing-baseline dance, the
    // second runs the steady path the timed run measures.
    add_fn(&lat, "m00", "warm_one", "1", false);
    add_fn(&lat, "m01", "warm_two", "2", false);

    let started = Instant::now();
    add_fn(&lat, "m02", "timed", "3", false);
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f64() <= 2.0,
        "gated add took {:.2}s on a 30-module crate",
        elapsed.as_secs_f64()
    );
}
