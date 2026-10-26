// Body and module-path helpers for building a change: where a source comes from,
// what a missing `--sig` defaults to, and how a module's raw preamble differs from
// a parsed body. Kept beside `change` because both answer the same question — what
// did the user mean — without deciding whether it is legal.

use super::super::{error, Env};
use super::super::report::Error;
use crate::cli::request::{Add, Source};
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::kind::Kind;
use crate::core::text::grammar;

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
        return format!("mod {}", stem(&add.name));
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

fn read_source(env: &Env<'_>, source: &Source, stdin: &str) -> Result<String, Error> {
    match source {
        Source::Stdin => Ok(stdin.to_string()),
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
