// `src/core/` is the inner circle: it decides, it never touches the outside
// world. This test is the dependency rule made executable — if core ever names
// a terminal, a file, a process or a socket, the build goes red here before a
// reviewer has to notice it.
//
// The reader is textual and line-oriented, which is enough for a repository
// whose core is a few hundred lines of pure data. It does not understand block
// comments or strings, so a *code* mention is a violation even inside a comment
// block, and a mention inside a string literal is not understood as such.
// Hazard: if core grows a legitimate reason to name `std::fs` in a doc comment,
// reword the comment rather than teaching this reader to distrust itself.

use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "crate::adapters",
    "crate::ports",
    "crate::app",
    "crate::cli",
    "std::fs",
    "std::process",
    "std::net",
    "std::env",
    "libc",
];

/// Every `(line_number, needle)` where `source` names something core may not.
fn violations_in(source: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        for needle in FORBIDDEN {
            if code.contains(needle) {
                found.push((index + 1, *needle));
            }
        }
    }
    found
}

/// Every `.rs` file anywhere under `directory`, in a stable order.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];

    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("read directory") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }

    found.sort();
    found
}

#[test]
fn reader_flags_an_outward_import() {
    let source = "use std::fs;\nfn load() {}\n";

    assert_eq!(violations_in(source), vec![(1, "std::fs")]);
}

#[test]
fn reader_flags_naming_an_outer_layer() {
    let source = "fn run() {\n    crate::adapters::terminal::open();\n}\n";

    assert_eq!(violations_in(source), vec![(2, "crate::adapters")]);
}

#[test]
fn reader_ignores_a_comment_mention() {
    let source = "// std::fs belongs to adapters, never here.\nuse super::model::ElId;\n";

    assert_eq!(violations_in(source), vec![]);
}

#[test]
fn core_names_nothing_outside_itself() {
    let mut offenders = Vec::new();
    for path in rust_files(Path::new("src/core")) {
        let source = std::fs::read_to_string(&path).expect("read core file");
        if let Some((line, needle)) = violations_in(&source).first() {
            offenders.push(format!("{}:{line} names {needle}", path.display()));
        }
    }

    assert!(offenders.is_empty(), "core reaches outward:\n{}", offenders.join("\n"));
}
