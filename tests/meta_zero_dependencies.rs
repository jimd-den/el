// The zero-dependency promise is the project's third headline claim, so it is
// checked by a test rather than by intention. This file holds the reader that
// turns a manifest's text into an answer, and the table that keeps the reader
// honest, plus the live assertion about this repository's own Cargo.toml.
//
// Hazard: the reader strips `#` comments and nothing else. It has no string
// awareness, so a `#` inside a quoted value would truncate the line. Manifests
// in this repo are hand-written and that case does not occur; if one ever does,
// the table gains a case before the reader gains code.
//
// It deliberately does not parse TOML: a full parser is a dependency, and this
// test exists to prove there are none.

/// True if `manifest` declares any crate as a dependency, dev-dependency or
/// build-dependency, in either inline or table form.
fn has_dependencies(manifest: &str) -> bool {
    let dependency_sections = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut inside_dependency_section = false;

    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let heading = line.trim_matches(|c| c == '[' || c == ']').trim();
            inside_dependency_section = dependency_sections
                .iter()
                .any(|section| heading == *section || heading.starts_with(&format!("{section}.")));
            continue;
        }
        if inside_dependency_section && line.contains('=') {
            return true;
        }
    }
    false
}

/// One table row: a manifest fragment and what the reader must say about it.
struct Case {
    what: &'static str,
    manifest: &'static str,
    expected: bool,
}

const CASES: &[Case] = &[
    Case {
        what: "an empty [dependencies] section adds nothing",
        manifest: "[package]\nname = \"el\"\n\n[dependencies]\n",
        expected: false,
    },
    Case {
        what: "a manifest with no dependency section adds nothing",
        manifest: "[package]\nname = \"el\"\n",
        expected: false,
    },
    Case {
        what: "an inline dependency is a dependency",
        manifest: "[dependencies]\nserde = \"1\"\n",
        expected: true,
    },
    Case {
        what: "a commented-out dependency is not a dependency",
        manifest: "[dependencies]\n# serde = \"1\"\n",
        expected: false,
    },
    Case {
        what: "a dev-dependency still has to be fetched",
        manifest: "[dev-dependencies]\nproptest = \"1\"\n",
        expected: true,
    },
    Case {
        what: "a build-dependency still has to be fetched",
        manifest: "[build-dependencies]\ncc = \"1\"\n",
        expected: true,
    },
    Case {
        what: "a table-form dependency is a dependency",
        manifest: "[dependencies.serde]\nversion = \"1\"\n",
        expected: true,
    },
];

#[test]
fn reads_each_manifests_dependency_set() {
    for case in CASES {
        assert_eq!(
            has_dependencies(case.manifest),
            case.expected,
            "{}",
            case.what
        );
    }
}

#[test]
fn this_repository_has_no_dependencies() {
    let manifest = std::fs::read_to_string("Cargo.toml").expect("read Cargo.toml");
    assert!(!has_dependencies(&manifest), "el must depend on nothing");
}
