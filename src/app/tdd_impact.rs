// Impact for `tdd impacted`, pure: which tests a production change
// touches, from the catalog's statements alone. The scan is the same
// whole-word rule `explain` uses, and it is honest about its limit:
// a changed element with no name (an impl block) cannot be scanned
// for, so impact is unavailable and the caller falls back to the
// full suite.

/// Whole-word mention: the name must not be glued to an identifier
/// character on either side.
fn mentions_word(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        let edge = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        edge(before) && edge(after)
    })
}

/// The impacted test ids: the target first, then every test whose
/// statements mention a changed production name, in catalog order.
/// `None` when a changed element has no name: impact is unavailable.
pub fn impacted_tests(
    changed: &[(String, Option<String>)],
    test_stmts: &[(String, Vec<String>)],
    target: &str,
) -> Option<Vec<String>> {
    let mut set = vec![target.to_string()];
    for (_id, name) in changed {
        let Some(name) = name else {
            return None;
        };
        for (test_id, stmts) in test_stmts {
            if test_id != target
                && !set.contains(test_id)
                && stmts.iter().any(|stmt| mentions_word(stmt, name))
            {
                set.push(test_id.clone());
            }
        }
    }
    Some(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mentioning_test_joins_the_target() {
        let changed = vec![("el-0003".to_string(), Some("helper".to_string()))];
        let test_stmts = vec![
            ("el-0007".to_string(), vec!["assert_eq!(helper(), 1);".to_string()]),
            ("el-0009".to_string(), vec!["assert_eq!(answer(), 43);".to_string()]),
        ];

        let set = impacted_tests(&changed, &test_stmts, "el-0005").unwrap();

        assert_eq!(set, vec!["el-0005".to_string(), "el-0007".to_string()]);
    }

    #[test]
    fn an_unnamed_change_makes_impact_unavailable() {
        let changed = vec![("el-0007".to_string(), None)];
        let test_stmts: Vec<(String, Vec<String>)> = vec![];

        assert_eq!(impacted_tests(&changed, &test_stmts, "el-0005"), None);
    }

    #[test]
    fn no_change_means_the_target_alone() {
        let changed: Vec<(String, Option<String>)> = vec![];
        let test_stmts = vec![("el-0007".to_string(), vec!["assert_eq!(helper(), 1);".to_string()])];

        let set = impacted_tests(&changed, &test_stmts, "el-0005").unwrap();

        assert_eq!(set, vec!["el-0005".to_string()]);
    }

    #[test]
    fn a_substring_mention_is_not_a_call() {
        let changed = vec![("el-0003".to_string(), Some("helper".to_string()))];
        let test_stmts = vec![("el-0007".to_string(), vec!["assert_eq!(helper2(), 1);".to_string()])];

        let set = impacted_tests(&changed, &test_stmts, "el-0005").unwrap();

        assert_eq!(set, vec!["el-0005".to_string()]);
    }
}
