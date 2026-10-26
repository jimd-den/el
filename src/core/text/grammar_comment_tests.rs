// The comment/attribute attachment rule, kept apart so the main grammar tests
// stay a readable list of tree shapes.

use super::*;

fn texts(text: &str) -> Vec<String> {
    parse(text).expect("parses").statements.into_iter().map(|s| s.text).collect()
}

#[test]
fn a_comment_or_attribute_attaches_to_the_following_statement() {
    assert_eq!(texts("///doc\nlet x = 1"), ["///doc\nlet x = 1;"]);
    assert_eq!(texts("// note\na()"), ["// note\na()"]);
    assert_eq!(texts("#[cfg(test)]\ntest()"), ["#[cfg(test)]\ntest()"]);
}

#[test]
fn a_trailing_comment_with_nothing_after_it_stays_a_statement() {
    assert_eq!(texts("a()\n// tail"), ["a();", "// tail"]);
}

