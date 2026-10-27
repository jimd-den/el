// Assembling a signature from plain words, and the return-type note.
// Kept beside `head` because both answer "what did the user mean" —
// but this one joins words where `head` dispatches, so neither file
// crosses the line budget alone.

use super::super::error;
use super::super::report::Error;
use crate::cli::request::Add;
use crate::core::model::body::Body;
use crate::core::model::kind::Kind;

/// Assemble a signature from plain words. `--sig` and the conflict and
/// empty checks live in `head::signature`; this answers only what each
/// kind's words join into.
pub fn assemble(add: &Add) -> Result<String, Error> {
    let words = add.takes.is_some()
        || add.returns.is_some()
        || add.decl_type.is_some()
        || add.value.is_some();
    match add.kind {
        Kind::Function | Kind::Test => {
            if add.decl_type.is_some() || add.value.is_some() {
                return Err(error::usage("--type and --value are not flags for fn or test"));
            }
            let params = add.takes.as_deref().unwrap_or("");
            let returns = add
                .returns
                .as_deref()
                .map(|ty| format!(" -> {ty}"))
                .unwrap_or_default();
            Ok(format!("fn {}({params}){returns}", add.name))
        }
        Kind::Const | Kind::Static => {
            if add.takes.is_some() || add.returns.is_some() {
                return Err(error::usage("--takes and --returns are not flags for const or static"));
            }
            let Some(ty) = add.decl_type.as_deref() else {
                return Err(error::needs_sig(add.kind));
            };
            let value = add.value.as_deref().map(|v| format!(" = {v}")).unwrap_or_default();
            Ok(format!("{} {}: {ty}{value}", add.kind.as_str(), add.name))
        }
        Kind::TypeAlias => {
            if add.takes.is_some() || add.returns.is_some() || add.value.is_some() {
                return Err(error::usage("only --type is a plain-words flag for type"));
            }
            let Some(ty) = add.decl_type.as_deref() else {
                return Err(error::needs_sig(add.kind));
            };
            Ok(format!("type {} = {ty}", add.name))
        }
        _ => {
            if words {
                return Err(error::usage(
                    "plain words are built for fn, test, const, static and type",
                ));
            }
            Ok(format!("{} {}", add.kind.as_str(), add.name))
        }
    }
}

/// Whether the add deserves the return-type note: a `fn` or `test`
/// declared without `--sig` and without `--returns`, with a body
/// given, whose last statement is a tail expression. The grammar's
/// own signal marks the tail: every non-tail statement gains a
/// `;`, the tail keeps none.
pub fn return_note(add: &Add, body: &Body) -> bool {
    if !matches!(add.kind, Kind::Function | Kind::Test) {
        return false;
    }
    if add.signature.is_some() || add.returns.is_some() || add.body.is_none() {
        return false;
    }
    let Some(last) = body.statements.last() else {
        return false;
    };
    !last.text.trim_end().ends_with(';')
}

/// The note itself, in the model's words.
pub const RETURN_NOTE: &str =
    "no --returns given, and the body ends in a value; if that value is the result, declare a return type";

#[cfg(test)]
#[path = "words_tests.rs"]
mod words_tests;
