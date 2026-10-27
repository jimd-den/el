// Addressing by name where el requires an id: the refusal names the exact
// resolution (`el find <name>`), and following it reaches the element — the
// loop an agent that remembers names but not ids actually runs.

use crate::program::{self, el, ok};

#[test]
fn show_by_name_names_the_find_command_that_resolves_it() {
    let built = program::build("show-by-name");

    let refused = el(&built.hello, &["show", "greet"]);

    assert_eq!(refused.status.code(), Some(6), "exit must be 6 (not found)");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(text.contains("el find greet"), "refusal names no resolution:\n{text}");

    let found = el(&built.hello, &["find", "greet"]);
    ok(&found, "el find greet");
    let found = String::from_utf8_lossy(&found.stdout);
    assert!(found.contains(&built.greet), "find resolves to the id:\n{found}");

    let shown = el(&built.hello, &["show", &built.greet]);
    ok(&shown, "el show <id>");
}
