// What an element is called, and the head it is created with. Kept beside
// `body` because both answer "what did the user mean" — a name and a file for a
// module, a declaration for an item — without deciding whether either is legal.
//
// The default head is the interesting part: `--sig` is an override, and every
// kind that can have a self-contained head gets one produced. A function without
// `--sig` is `fn name()`, not `fn name`, because only the former is a
// declaration. `const`, `static` and `type` need a type the tool cannot invent,
// so they are refused before staging rather than handed to the gate as invalid
// Rust.
//
// It deliberately does not parse a signature or check it against the kind: the
// model's `Element::create` owns that, and a second opinion here would drift.

use super::super::error;
use super::words;
use super::super::report::Error;
use crate::cli::request::Add;
use crate::core::model::kind::Kind;

/// The file a module reference names. `catalog`, `src/catalog.rs`,
/// `core/plan` and `core::plan` all name one file each, because the catalog
/// stores file paths and the tree derives nodes from them: a bare name stored
/// verbatim becomes a second identity for the same module — a tree node no
/// file renders, and a seeded module element whose alias collides with the
/// one the user just added. Normalizing at the command-line boundary keeps
/// one module one path. An empty reference is returned unchanged, for the
/// layers below to refuse as an unknown module rather than inventing one.
pub fn normalize_module(reference: &str) -> String {
    let stem = reference.strip_prefix("src/").unwrap_or(reference);
    let stem = stem.strip_suffix(".rs").unwrap_or(stem);
    let stem = stem.replace("::", "/");
    if stem.is_empty() {
        return reference.to_string();
    }
    format!("src/{stem}.rs")
}

pub fn root_module(env: &super::super::Env<'_>) -> String {
    let lib = env.fs.exists("src/lib.rs").unwrap_or(false);
    let main = env.fs.exists("src/main.rs").unwrap_or(false);
    if lib && !main {
        "src/lib.rs".to_string()
    } else {
        "src/main.rs".to_string()
    }
}

pub fn signature(add: &Add) -> Result<String, Error> {
    if add.kind == Kind::Module {
        return Ok(format!("mod {}", stem(&normalize_module(&add.name))));
    }
    let words = add.takes.is_some()
        || add.returns.is_some()
        || add.decl_type.is_some()
        || add.value.is_some();
    if add.signature.is_some() && words {
        return Err(error::usage(
            "give either --sig or the plain words (--takes, --returns, --type, --value), not both",
        ));
    }
    if let Some(signature) = &add.signature {
        return Ok(signature.clone());
    }
    for (flag, value) in [
        ("--takes", &add.takes),
        ("--returns", &add.returns),
        ("--type", &add.decl_type),
        ("--value", &add.value),
    ] {
        if matches!(value, Some(text) if text.trim().is_empty()) {
            return Err(error::usage(format!("{flag} needs a value")));
        }
    }
    words::assemble(add)
}

fn stem(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".rs").unwrap_or(file)
}


#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn every_spelling_of_a_module_names_its_file() {
        assert_eq!(normalize_module("catalog"), "src/catalog.rs");
        assert_eq!(normalize_module("src/catalog.rs"), "src/catalog.rs");
        assert_eq!(normalize_module("core/plan"), "src/core/plan.rs");
        assert_eq!(normalize_module("core::plan"), "src/core/plan.rs");
        assert_eq!(normalize_module("src/main.rs"), "src/main.rs");
        assert_eq!(normalize_module("main"), "src/main.rs");
        assert_eq!(normalize_module("src/a/mod.rs"), "src/a/mod.rs");
    }

    #[test]
    fn a_default_function_signature_has_parentheses() {
        let add = add(Kind::Function, "main", None);

        assert_eq!(signature(&add).expect("signature"), "fn main()");
    }

    #[test]
    fn a_signature_flag_overrides_the_default() {
        let add = add(Kind::Function, "main", Some("fn main(argc: i32)"));

        assert_eq!(signature(&add).expect("signature"), "fn main(argc: i32)");
    }

    #[test]
    fn a_type_the_tool_cannot_invent_needs_a_sig() {
        let add = add(Kind::Const, "LIMIT", None);

        assert_eq!(
            signature(&add).expect_err("refused").code,
            "usage.missing_argument"
        );
    }

    #[test]
    fn a_sig_and_plain_words_together_are_refused() {
        let mut add = add(Kind::Function, "greet", Some("fn greet()"));
        add.takes = Some("name: &str".to_string());

        let refused = signature(&add).expect_err("refused");
        assert_eq!(refused.code, "usage.conflict");
    }

    #[test]
    fn an_empty_plain_word_is_refused() {
        let mut add = add(Kind::Function, "greet", None);
        add.takes = Some("".to_string());

        let refused = signature(&add).expect_err("refused");
        assert_eq!(refused.code, "usage.conflict");
    }
}
