// Tests for `words`: the plain-words assembly and the return-type note.
// Sibling of the module they exercise.

use super::*;
use crate::cli::request::Add;
use crate::core::model::kind::Kind;

fn add(kind: Kind, name: &str, signature: Option<&str>) -> Add {
    Add {
        kind,
        name: name.to_string(),
        module: None,
        visibility: None,
        signature: signature.map(str::to_string),
        takes: None,
        returns: None,
        decl_type: None,
        value: None,
        body: None,
        doc: None,
        preamble: None,
        after: None,
        before: None,
        at: None,
    }
}

    fn words(
        kind: Kind,
        name: &str,
        takes: Option<&str>,
        returns: Option<&str>,
        decl_type: Option<&str>,
        value: Option<&str>,
    ) -> Add {
        Add {
            kind,
            name: name.to_string(),
            module: None,
            visibility: None,
            signature: None,
            takes: takes.map(str::to_string),
            returns: returns.map(str::to_string),
            decl_type: decl_type.map(str::to_string),
            value: value.map(str::to_string),
            body: None,
            doc: None,
            preamble: None,
            after: None,
            before: None,
            at: None,
        }
    }

    #[test]
    fn plain_words_assemble_a_function_signature() {
        let add = words(Kind::Function, "greet", Some("name: &str"), Some("String"), None, None);

        assert_eq!(assemble(&add).expect("signature"), "fn greet(name: &str) -> String");
    }

    #[test]
    fn plain_words_assemble_a_function_without_a_return_type() {
        let add = words(Kind::Function, "greet", Some("name: &str"), None, None, None);

        assert_eq!(assemble(&add).expect("signature"), "fn greet(name: &str)");
    }

    #[test]
    fn plain_words_assemble_a_test_without_a_sig() {
        let add = words(Kind::Test, "checks_answer", None, None, None, None);

        assert_eq!(assemble(&add).expect("signature"), "fn checks_answer()");
    }

    #[test]
    fn plain_words_assemble_a_test_with_a_return_type() {
        let add = words(Kind::Test, "checks_answer", None, Some("Result<(), String>"), None, None);

        assert_eq!(
            assemble(&add).expect("signature"),
            "fn checks_answer() -> Result<(), String>"
        );
    }

    #[test]
    fn plain_words_assemble_a_const_with_a_type_and_a_value() {
        let add = words(Kind::Const, "LIMIT", None, None, Some("usize"), Some("10"));

        assert_eq!(assemble(&add).expect("signature"), "const LIMIT: usize = 10");
    }

    #[test]
    fn plain_words_assemble_a_const_with_a_type_but_no_value() {
        let add = words(Kind::Const, "LIMIT", None, None, Some("usize"), None);

        assert_eq!(assemble(&add).expect("signature"), "const LIMIT: usize");
    }

    #[test]
    fn plain_words_assemble_a_type_alias() {
        let add = words(Kind::TypeAlias, "Id", None, None, Some("u32"), None);

        assert_eq!(assemble(&add).expect("signature"), "type Id = u32");
    }

    #[test]
    fn a_const_without_a_type_names_the_plain_words_flag() {
        let add = words(Kind::Const, "LIMIT", None, None, None, None);

        let refused = assemble(&add).expect_err("refused");
        assert_eq!(refused.code, "usage.missing_argument");
        assert!(refused.message.contains("--type"), "message names no flag: {}", refused.message);
    }

    #[test]
    fn plain_words_for_the_wrong_kind_are_refused() {
        let fn_with_a_type = words(Kind::Function, "greet", None, None, Some("u32"), None);
        let const_with_params = words(Kind::Const, "LIMIT", Some("x: u32"), None, None, None);

        assert!(assemble(&fn_with_a_type).is_err(), "a fn must refuse --type");
        assert!(assemble(&const_with_params).is_err(), "a const must refuse --takes");
    }

    fn body_with(text: &str) -> crate::core::model::body::Body {
        use crate::core::model::body::BodyStmt;
        crate::core::model::body::Body {
            statements: vec![BodyStmt { text: text.to_string(), children: Vec::new() }],
        }
    }

    #[test]
    fn a_tail_body_without_a_return_type_needs_the_note() {
        let mut add = words(Kind::Function, "answer", None, None, None, None);
        add.body = Some(crate::cli::request::Source::Literal("0".to_string()));

        assert!(return_note(&add, &body_with("0")));
    }

    #[test]
    fn a_body_ending_in_a_semicolon_needs_no_note() {
        let mut add = words(Kind::Function, "answer", None, None, None, None);
        add.body = Some(crate::cli::request::Source::Literal("foo();".to_string()));

        assert!(!return_note(&add, &body_with("foo();")));
    }

    #[test]
    fn a_body_that_was_never_given_needs_no_note() {
        let add = words(Kind::Function, "answer", None, None, None, None);

        assert!(!return_note(&add, &body_with("todo!()")));
    }

    #[test]
    fn a_declared_return_type_needs_no_note() {
        let mut add = words(Kind::Function, "answer", None, Some("u32"), None, None);
        add.body = Some(crate::cli::request::Source::Literal("0".to_string()));

        assert!(!return_note(&add, &body_with("0")));
    }

    #[test]
    fn a_sig_declared_function_needs_no_note() {
        let mut add = add(Kind::Function, "answer", Some("fn answer()"));
        add.body = Some(crate::cli::request::Source::Literal("0".to_string()));

        assert!(!return_note(&add, &body_with("0")));
    }
