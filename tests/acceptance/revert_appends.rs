// FR-31: revert appends a new entry whose tree equals the target. History is not
// rewritten; the target's digest is visible in the new entry.

use crate::program::{self, el, log_json, ok};

fn number(value: Option<&el::core::text::json::Json>) -> u64 {
    value.and_then(|v| v.as_number()).expect("number")
}

#[test]
fn revert_appends_an_entry_with_the_target_tree() {
    let built = program::build("revert");
    let before = log_json(&built.hello, false);
    let target = before
        .iter()
        .find(|entry| number(entry.get("seq")) == 3)
        .expect("seq 3");
    let target_to = target.get("to").and_then(|v| v.as_str()).expect("to").to_string();
    let head = number(before[0].get("seq"));

    ok(&el(&built.hello, &["revert", "3"]), "revert");

    let after = log_json(&built.hello, false);
    assert_eq!(number(after[0].get("seq")), head + 1);
    assert_eq!(after[0].get("op").and_then(|v| v.as_str()), Some("revert"));
    assert_eq!(after[0].get("target").and_then(|v| v.as_str()), Some("3"));
    assert_eq!(after[0].get("to").and_then(|v| v.as_str()), Some(target_to.as_str()));
}
