// The skill is the surface an agent reads instead of the source: one page, no
// crate needed, and every verb and flag the parser dispatches appears on it. A
// golden file pins the exact text so an added flag cannot silently go undocumented.

use crate::program::{el, fresh, ok};

const GUIDE: &str = include_str!("../fixtures/golden/skill.txt");
const MAX_LINES: usize = 150;

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn skill_is_one_short_page() {
    let root = fresh("skill-lines");

    let output = el(&root, &["skill"]);

    ok(&output, "skill");
    let text = stdout(&output);
    assert!(
        text.lines().count() <= MAX_LINES,
        "skill is {} lines, over {MAX_LINES}",
        text.lines().count()
    );
}

#[test]
fn skill_names_every_verb_and_flag() {
    let root = fresh("skill-surface");

    let text = stdout(&el(&root, &["skill"]));

    for verb in el::cli::parse::VERBS {
        assert!(text.split_whitespace().any(|word| word == *verb), "skill omits verb {verb}");
    }
    for flag in el::cli::args::VALUE_FLAGS.iter().chain(el::cli::args::BOOL_FLAGS) {
        assert!(text.contains(flag), "skill omits flag {flag}");
    }
}

#[test]
fn skill_is_pinned_by_a_golden_file() {
    let root = fresh("skill-golden");

    let text = stdout(&el(&root, &["skill"]));

    assert_eq!(text, GUIDE, "skill changed; update tests/fixtures/golden/skill.txt");
}

#[test]
fn help_prints_the_same_page_as_skill() {
    let root = fresh("skill-help");

    let skill_out = el(&root, &["skill"]);
    let help_out = el(&root, &["--help"]);
    ok(&skill_out, "skill");
    ok(&help_out, "--help");

    assert_eq!(stdout(&help_out), stdout(&skill_out));
}
