// Every error code the source constructs must have a hint in the one table, or a
// refusal lands with no next command. This walks `src/` at test time: a new code
// is caught by the gate, not by a reviewer remembering.

use std::fs;
use std::path::Path;

fn codes(dir: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read src") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            codes(&path, found);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read file");
        for marker in ["code: \"", "Error::new(\"", "Usage::new(\""] {
            let mut rest = text.as_str();
            while let Some(at) = rest.find(marker) {
                let after = &rest[at + marker.len()..];
                if let Some(end) = after.find('"') {
                    let code = &after[..end];
                    let compiler_code = code.starts_with('E')
                        && code[1..].chars().all(|c| c.is_ascii_digit());
                    if !code.is_empty() && !code.contains(' ') && !compiler_code {
                        found.push(code.to_string());
                    }
                }
                rest = after;
            }
        }
    }
}

#[test]
fn every_constructed_code_has_a_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    codes(&root.join("src"), &mut found);
    found.sort();
    found.dedup();

    let missing: Vec<&String> = found
        .iter()
        .filter(|code| !el::app::report::HINTS.iter().any(|(known, _)| known == *code))
        .collect();

    assert!(missing.is_empty(), "codes without a hint: {missing:?}");
    assert!(!found.is_empty());
}
