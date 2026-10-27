// A function whose name is also its module's — `greet::greet` — is the case a
// token rewrite gets wrong: it renames both segments and the call stops resolving.
// The catalog knows the renamed element's kind, so a function rename leaves the
// qualifier alone and a module rename touches only the qualifier.

use crate::program::{build, el, ok};
use std::fs;

#[test]
fn renaming_a_function_keeps_the_module_it_lives_in() {
    let built = build("rename-qualifier");

    let output = el(&built.hello, &["edit", &built.greet, "--rename", "salute"]);

    ok(&output, "rename greet");
    let main = fs::read_to_string(built.hello.join("src/main.rs")).expect("main");
    assert!(main.contains("greet::salute(\"world\")"), "{main}");
    ok(&el(&built.hello, &["run"]), "run after rename");
}
