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
