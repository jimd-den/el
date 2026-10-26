// The filtering, field and refusal half of the read surface: `ls` narrows by
// module and kind, `show` speaks only in ids, and an alias is a named refusal
// rather than a guess. `--json` is the same report through the same envelope.

use crate::program::{self, el, ok};

#[test]
fn ls_filters_by_module_and_kind_and_marks_the_body_mode() {
    let built = program::build("ls-filters");

    let functions = el(&built.hello, &["ls", "--kind", "fn"]);
    ok(&functions, "ls --kind fn");
    let functions = String::from_utf8_lossy(&functions.stdout);
    assert!(functions.contains(&built.greet), "{functions}");
    assert!(functions.contains(&built.main), "{functions}");
    assert!(!functions.contains(" mod "), "a kind filter included a module:\n{functions}");

    let module = el(&built.hello, &["ls", "--module", "src/greet.rs"]);
    ok(&module, "ls --module");
    let module = String::from_utf8_lossy(&module.stdout);
    assert!(module.contains(&built.greet), "{module}");
    assert!(!module.contains(&built.main), "{module}");

    let opaque = el(&built.hello, &["ls", "--opaque"]);
    ok(&opaque, "ls --opaque");
    assert!(String::from_utf8_lossy(&opaque.stdout).lines().count() == 1, "only the header");

    let json = el(&built.hello, &["ls", "--json"]);
    ok(&json, "ls --json");
    let line = String::from_utf8_lossy(&json.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("our own JSON parses");
    assert!(parsed.get("data").and_then(|v| v.as_array()).is_some_and(|rows| !rows.is_empty()), "{line}");
}

#[test]
fn show_refuses_an_alias_and_an_unknown_id_with_exit_six() {
    let built = program::build("show-refusals");

    for bad in ["greet::fn/greet", "el-9999"] {
        let output = el(&built.hello, &["show", bad, "--json"]);
        assert_eq!(output.status.code(), Some(6), "{bad}");
        let line = String::from_utf8_lossy(&output.stdout);
        let parsed = el::core::text::json::parse(line.trim()).expect("our own JSON parses");
        assert_eq!(
            parsed
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(|code| code.as_str()),
            Some("notfound.element"),
            "{bad}: {line}"
        );
    }
}

#[test]
fn ls_kind_stmt_and_show_a_statement_use_the_statement_id() {
    let built = program::build("stmt-read");
    let show = el(&built.hello, &["show", &built.greet, "--json"]);
    ok(&show, "show greet");
    let line = String::from_utf8_lossy(&show.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("json");
    let statements = parsed
        .get("data")
        .and_then(|d| d.get("statements"))
        .and_then(|v| v.as_array())
        .expect("statements");
    let statement = statements[0].as_str().expect("id").to_string();

    let ls = el(&built.hello, &["ls", "--kind", "stmt"]);
    ok(&ls, "ls --kind stmt");
    assert!(String::from_utf8_lossy(&ls.stdout).contains(&statement));

    let shown = el(&built.hello, &["show", &statement, "--json"]);
    ok(&shown, "show statement");
    let line = String::from_utf8_lossy(&shown.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("json");
    let data = parsed.get("data").expect("data");
    assert_eq!(data.get("kind").and_then(|v| v.as_str()), Some("stmt"));
    assert_eq!(data.get("owner").and_then(|v| v.as_str()), Some(built.greet.as_str()));
}

#[test]
fn show_json_has_every_documented_field_and_the_source() {
    let built = program::build("show-json");

    let output = el(&built.hello, &["show", &built.greet, "--statements", "--source", "--json"]);
    ok(&output, "el show --json");
    let line = String::from_utf8_lossy(&output.stdout);
    let parsed = el::core::text::json::parse(line.trim()).expect("our own JSON parses");
    let data = parsed.get("data").expect("show payload");

    for field in [
        "id", "kind", "alias", "module", "visibility", "signature", "doc", "mode",
        "sequence", "statements", "source",
    ] {
        assert!(data.get(field).is_some(), "missing {field}: {line}");
    }
    assert_eq!(data.get("id").and_then(|v| v.as_str()), Some(built.greet.as_str()));
    assert_eq!(
        data.get("doc").and_then(|v| v.as_str()),
        Some("/// Greets by name."),
        "{line}"
    );
    assert!(
        data.get("source").and_then(|v| v.as_str()).is_some_and(|s| s.contains("format!")),
        "{line}"
    );
}
