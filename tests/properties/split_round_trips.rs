use el::core::text::split::{join, split};
use std::fs;
use std::path::{Path, PathBuf};

// FR-65, over a checked-in corpus rather than an example: every body `el` might
// meet either splits losslessly or is refused, and the refusal list is a fixture
// so that a future "improvement" which starts guessing fails here.
//
// The corpus is deliberately adversarial — raw strings holding `}`, macros whose
// tokens look unbalanced, labelled loops, lifetimes beside char literals, block
// comments containing `*/` inside a string. None of them may produce a split
// whose rejoin differs from the input, and none of the refusal files may produce
// a split at all.

fn bodies(directory: &Path) -> Vec<(String, String)> {
    let mut found: Vec<PathBuf> = fs::read_dir(directory)
        .expect("fixture directory")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    found.sort();

    found
        .into_iter()
        .map(|path| {
            let name = path.file_name().expect("name").to_string_lossy().to_string();
            let text = fs::read_to_string(&path).expect("read body");
            (name, text)
        })
        .collect()
}

#[test]
fn every_corpus_body_splits_losslessly() {
    let corpus = bodies(Path::new("tests/fixtures/bodies"));
    assert_eq!(corpus.len(), 40, "the corpus is the fixture, keep it whole");

    for (name, body) in corpus {
        let pieces = split(&body).unwrap_or_else(|| panic!("refused a splittable body: {name}"));

        assert_eq!(join(&pieces), body, "round trip lost bytes: {name}");
    }
}

#[test]
fn every_corpus_body_splits_idempotently() {
    for (name, body) in bodies(Path::new("tests/fixtures/bodies")) {
        let pieces = split(&body).expect("splittable");
        let rejoined = join(&pieces);
        let again = split(&rejoined).expect("still splittable");

        assert_eq!(pieces, again, "second split differs: {name}");
    }
}

#[test]
fn every_unprovable_body_is_refused() {
    let refusals = bodies(Path::new("tests/fixtures/bodies/refused"));
    assert_eq!(refusals.len(), 9, "the refusal list is the fixture, keep it whole");

    for (name, body) in refusals {
        assert_eq!(split(&body), None, "guessed at a boundary instead of refusing: {name}");
    }
}
