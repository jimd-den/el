// A refusal is only useful if it says what to type next. The table is one place,
// so a new code cannot be published without a next command, and every command it
// names is checked against the real parser rather than trusted.

use super::*;

fn argv(command: &str) -> Vec<String> {
    command.split_whitespace().skip(1).map(str::to_string).collect()
}

#[test]
fn every_hint_is_a_command_the_parser_accepts() {
    for (code, hint) in HINTS {
        assert!(hint.starts_with("el "), "{code}: {hint}");
        assert!(
            crate::cli::parse(&argv(hint)).is_ok(),
            "{code}: {hint} is not valid argv"
        );
    }
}

#[test]
fn every_plan_code_has_its_own_hint() {
    for code in crate::core::plan::Code::all() {
        assert!(
            HINTS.iter().any(|(known, _)| *known == code.as_str()),
            "no hint for {}",
            code.as_str()
        );
    }
}

#[test]
fn an_unlisted_code_falls_back_to_the_help_page() {
    assert_eq!(hint_for("something.new"), "el help");
}

#[test]
fn the_error_carries_the_hint_for_its_code() {
    let error = Error {
        code: "gate.rejected",
        exit: Exit::Gate,
        message: "1 new error".to_string(),
    };

    assert_eq!(error.hint(), "el diff");
}
