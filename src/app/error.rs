// One place where a lower layer's error becomes the report the edge can print.
// Keeping the mapping here means `Exit` is chosen once per failure class, not at
// every `return`, and the envelope's stable code is never invented at a call site.

pub use super::report::Error;
use super::Exit;
use crate::ports::{fs, journal};

pub fn store(error: journal::Error) -> Error {
    Error {
        code: error.code,
        exit: Exit::Env,
        message: error.message,
    }
}

pub fn filesystem(error: fs::Error) -> Error {
    Error {
        code: "env.io",
        exit: Exit::Env,
        message: error.to_string(),
    }
}

pub fn corrupt(message: impl Into<String>) -> Error {
    Error {
        code: "env.corrupt",
        exit: Exit::Env,
        message: message.into(),
    }
}

pub fn not_a_crate(root: &str) -> Error {
    Error {
        code: "env.not_a_crate",
        exit: Exit::Env,
        message: format!("{root} is not an el crate; run `el new` or `el init` first"),
    }
}

pub fn unsupported(command: &'static str, what: &str) -> Error {
    Error {
        code: "usage.unsupported",
        exit: Exit::Usage,
        message: format!("{command} does not support {what} yet"),
    }
}

pub fn bad_body(error: crate::core::text::grammar::Error) -> Error {
    Error {
        code: "model.unbalanced",
        exit: Exit::Violation,
        message: error.to_string(),
    }
}
