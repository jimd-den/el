// Body and module-path helpers for building a change: where a source comes from,
// what a missing `--sig` defaults to, and how a module's raw preamble differs from
// a parsed body. Kept beside `change` because both answer the same question — what
// did the user mean — without deciding whether it is legal.

use super::super::{error, Env};
use super::super::report::Error;
use crate::cli::request::{Add, Source};
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::element::BodyMode;
use crate::core::model::kind::Kind;
use crate::core::plan::policy::Split;
use crate::core::text::grammar;

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

pub fn root_module(env: &Env<'_>) -> String {
    let lib = env.fs.exists("src/lib.rs").unwrap_or(false);
    let main = env.fs.exists("src/main.rs").unwrap_or(false);
    if lib && !main {
        "src/lib.rs".to_string()
    } else {
        "src/main.rs".to_string()
    }
}

pub fn signature(add: &Add) -> String {
    if add.kind == Kind::Module {
        return format!("mod {}", stem(&normalize_module(&add.name)));
    }
    add.signature
        .clone()
        .unwrap_or_else(|| format!("{} {}", add.kind.as_str(), add.name))
}

fn stem(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".rs").unwrap_or(file)
}

pub fn preamble(env: &Env<'_>, add: &Add, stdin: &str) -> Result<Body, Error> {
    let text = match add.preamble.as_ref() {
        Some(source) => read_source(env, source, stdin)?,
        None => format!("// {} — add a preamble with `el edit`.", add.name),
    };
    Ok(Body {
        statements: text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| BodyStmt {
                text: line.trim().to_string(),
                children: Vec::new(),
            })
            .collect(),
    })
}

/// A body for a statement verb: always split, because a statement is the unit
/// being addressed.
pub fn body(
    env: &Env<'_>,
    source: Option<&Source>,
    stdin: &str,
    needs_body: bool,
) -> Result<Body, Error> {
    match source {
        Some(source) => grammar::parse(&read_source(env, source, stdin)?).map_err(error::bad_body),
        None if needs_body => Ok(Body {
            statements: vec![BodyStmt {
                text: "todo!()".to_string(),
                children: Vec::new(),
            }],
        }),
        None => Ok(Body::default()),
    }
}

/// A body for an element, with the policy's split mode. `never` stores one
/// opaque blob; `auto` falls back to opaque when the grammar cannot prove a
/// boundary; `always` refuses instead of falling back.
pub fn element_body(
    env: &Env<'_>,
    source: Option<&Source>,
    stdin: &str,
    needs_body: bool,
    split: Split,
) -> Result<(Body, BodyMode), Error> {
    let Some(source) = source else {
        let body = if needs_body {
            Body {
                statements: vec![BodyStmt {
                    text: "todo!()".to_string(),
                    children: Vec::new(),
                }],
            }
        } else {
            Body::default()
        };
        return Ok((body, BodyMode::Split));
    };
    let text = read_source(env, source, stdin)?;
    match split {
        Split::Never => Ok((opaque(&text), BodyMode::Opaque)),
        Split::Always => Ok((grammar::parse(&text).map_err(error::bad_body)?, BodyMode::Split)),
        Split::Auto => match grammar::parse(&text) {
            Ok(body) => Ok((body, BodyMode::Split)),
            Err(_) => Ok((opaque(&text), BodyMode::Opaque)),
        },
    }
}

fn opaque(text: &str) -> Body {
    Body {
        statements: vec![BodyStmt {
            text: text.trim_end().to_string(),
            children: Vec::new(),
        }],
    }
}

fn read_source(env: &Env<'_>, source: &Source, stdin: &str) -> Result<String, Error> {
    match source {
        Source::Stdin => Ok(stdin.to_string()),
        Source::Literal(text) => Ok(text.clone()),
        Source::File(path) => {
            let bytes = env
                .fs
                .read(path)
                .map_err(error::filesystem)?
                .ok_or_else(|| Error {
                    code: "env.io",
                    exit: super::super::Exit::Env,
                    message: format!("{path} does not exist"),
                })?;
            String::from_utf8(bytes).map_err(|_| error::corrupt(format!("{path} is not UTF-8")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
