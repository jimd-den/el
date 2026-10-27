// The page and the parser must agree in both directions: a verb the parser
// dispatches but the page omits is undiscoverable, and a verb the page promises
// but the parser refuses is a lie. The flag tables are read directly.

use super::*;
use crate::cli::args::{BOOL_FLAGS, VALUE_FLAGS};
use crate::cli::parse::VERBS;

#[test]
fn every_dispatched_verb_has_a_line() {
    for verb in VERBS {
        assert!(
            VERB_LINES.iter().any(|(name, _, _)| name == verb),
            "skill omits verb {verb}"
        );
    }
}

#[test]
fn every_line_names_a_verb_the_parser_accepts() {
    for (name, _, _) in VERB_LINES {
        assert!(VERBS.contains(name), "skill promises unknown verb {name}");
    }
}

#[test]
fn every_flag_the_parser_reads_is_on_the_page() {
    for flag in VALUE_FLAGS.iter().chain(BOOL_FLAGS) {
        assert!(
            FLAG_LINES
                .iter()
                .any(|(line, _)| line.split_whitespace().next() == Some(*flag)),
            "skill omits flag {flag}"
        );
    }
}

#[test]
fn the_page_stays_within_its_published_ceiling() {
    assert!(guide().lines().count() <= MAX_LINES);
}

#[test]
fn the_page_groups_verbs_by_task() {
    let page = guide();
    let groups: &[(&str, &[&str])] = &[
        ("write", &["new", "init", "add", "batch", "apply", "edit", "rm", "mv"]),
        ("read", &["ls", "tree", "show", "find", "explain", "diff", "log"]),
        ("history", &["undo", "redo", "revert"]),
        ("verify", &["run", "check", "doctor"]),
        ("meta", &["version", "skill", "help"]),
    ];
    for (index, (group, verbs)) in groups.iter().enumerate() {
        let heading = page
            .find(&format!("\n{group}\n"))
            .unwrap_or_else(|| panic!("page has no {group} heading"));
        let next = groups[index + 1..]
            .iter()
            .find_map(|(later, _)| page.find(&format!("\n{later}\n")));
        for verb in *verbs {
            let line = page
                .find(&format!("\n  {verb} "))
                .unwrap_or_else(|| panic!("page has no {verb} line"));
            assert!(line > heading, "{verb} is not under {group}");
            if let Some(next) = next {
                assert!(line < next, "{verb} is not under {group}");
            }
        }
    }
    for (name, _, _) in VERB_LINES {
        let homes = groups
            .iter()
            .filter(|(_, verbs)| verbs.contains(name))
            .count();
        assert_eq!(homes, 1, "verb {name} must sit in exactly one group");
    }
}

#[test]
fn the_page_is_not_empty_and_has_no_duplicate_verbs() {
    let page = guide();
    assert!(page.contains("usage: el"));
    for (index, (name, _, _)) in VERB_LINES.iter().enumerate() {
        assert!(
            !VERB_LINES[..index].iter().any(|(earlier, _, _)| earlier == name),
            "duplicate verb line {name}"
        );
    }
}

#[test]
fn the_kinds_line_lists_exactly_the_real_kinds() {
    use crate::core::model::kind::Kind;

    let page = guide();
    let lines: Vec<&str> = page.lines().collect();
    let kinds_at = lines.iter().position(|line| *line == "kinds").expect("kinds section");
    let mut listed: Vec<&str> = lines[kinds_at + 1].split_whitespace().collect();
    listed.sort_unstable();
    let mut real: Vec<&str> = Kind::all().iter().map(|kind| kind.as_str()).collect();
    real.sort_unstable();
    assert_eq!(listed, real, "kinds line promises a kind the parser refuses, or omits one");
}
