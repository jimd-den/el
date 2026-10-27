// `el init`: adoption, not generation. Every file under `src/` becomes one opaque
// module, so the catalog can address, check and edit it at file granularity while
// every byte on disk stays what the author wrote. The manifest is left alone; an
// adoption that rewrote it would not be adoption.
//
// It writes the store once, because there is no change to gate: the tree it
// records is the tree already on disk, checked byte for byte before the commit.

mod commit;
use super::error::{self, Error};
use super::report::Report;
use super::{Env, Exit};
use crate::core::model::catalog::Catalog;
use crate::core::model::element::{BodyMode, NewElement, Visibility};
use crate::core::model::kind::Kind;
use crate::core::render::render;

pub fn init(env: &Env<'_>, dry_run: bool, force: bool) -> Report {
    if dry_run {
        return Report::fail("init", error::unsupported("init", "--dry-run"));
    }
    if force {
        return Report::fail("init", error::unsupported("init", "--force"));
    }
    if env.fs.exists(".el/version").unwrap_or(false) {
        return Report::fail(
            "init",
            Error {
                code: "usage.exists",
                exit: Exit::Usage,
                message: format!("{} already holds an el crate", env.root),
            },
        );
    }
    if !env.fs.exists("Cargo.toml").unwrap_or(false) {
        return Report::fail("init", not_a_project(env));
    }
    let files = match sources(env) {
        Ok(files) => files,
        Err(error) => return Report::fail("init", error),
    };
    let (catalog, texts) = match adopt(env, &files) {
        Ok(adopted) => adopted,
        Err(error) => return Report::fail("init", error),
    };
    if let Err(error) = verify(env, &catalog, &texts) {
        return Report::fail("init", error);
    }
    match commit::commit(env, &catalog, &texts) {
        Ok(count) => {
            let mut report = Report::new("init");
            report.lines = vec![format!("adopted {count} files as opaque modules")];
            report
        }
        Err(error) => Report::fail("init", error),
    }
}

fn sources(env: &Env<'_>) -> Result<Vec<String>, Error> {
    let listed = env.fs.list("src").map_err(error::filesystem)?;
    let files: Vec<String> = listed
        .into_iter()
        .filter(|path| path.ends_with(".rs") && !path.starts_with("tmp/"))
        .collect();
    if files.is_empty() {
        return Err(Error {
            code: "env.not_a_project",
            exit: Exit::Env,
            message: "src/ holds no .rs files to adopt".to_string(),
        });
    }
    Ok(files)
}

fn adopt(env: &Env<'_>, files: &[String]) -> Result<(Catalog, Vec<(String, String)>), Error> {
    let mut catalog = Catalog::new();
    let mut texts = Vec::with_capacity(files.len());
    for path in files {
        let bytes = env.fs.read(path).map_err(error::filesystem)?;
        let Some(bytes) = bytes else {
            return Err(error::corrupt(format!("{path} disappeared while adopting")));
        };
        let text = String::from_utf8(bytes)
            .map_err(|_| error::corrupt(format!("{path} is not UTF-8")))?;
        let name = module_name(path);
        let signature = format!("mod {name}");
        let id = catalog
            .add_element(&NewElement {
                kind: Kind::Module,
                module: path,
                signature: &signature,
                visibility: Visibility::Private,
            })
            .map_err(|failure| model_error(path, failure))?;
        catalog
            .add_stmt(id, &text)
            .map_err(|failure| model_error(path, failure))?;
        if let Some(element) = catalog.find_mut(id) {
            element.mode = BodyMode::Opaque;
        }
        texts.push((path.clone(), text));
    }
    Ok((catalog, texts))
}

fn verify(
    env: &Env<'_>,
    catalog: &Catalog,
    texts: &[(String, String)],
) -> Result<(), Error> {
    let projection = render(catalog, &env.crate_name);
    let known: std::collections::BTreeSet<&str> =
        texts.iter().map(|(path, _)| path.as_str()).collect();
    for file in projection.files.iter().filter(|file| file.path.ends_with(".rs")) {
        if !known.contains(file.path.as_str()) {
            return Err(error::corrupt(format!(
                "adopting would invent {}; this crate layout is not supported",
                file.path
            )));
        }
    }
    for (path, text) in texts {
        let file = projection
            .file(path)
            .ok_or_else(|| error::corrupt(format!("{path} did not render")))?;
        if file.text != *text {
            return Err(error::corrupt(format!(
                "adopting {path} would change its bytes; refusing"
            )));
        }
    }
    Ok(())
}

fn module_name(path: &str) -> String {
    let leaf = path.rsplit('/').next().unwrap_or(path);
    let stem = leaf.strip_suffix(".rs").unwrap_or(leaf);
    if stem == "mod" {
        path.rsplit('/').nth(1).unwrap_or("mod").to_string()
    } else {
        stem.to_string()
    }
}

fn model_error(path: &str, failure: crate::core::model::catalog::Error) -> Error {
    Error {
        code: match failure {
            crate::core::model::catalog::Error::DuplicateAlias { .. } => "model.duplicate_alias",
            crate::core::model::catalog::Error::BadSignature { .. } => "model.bad_ident",
            crate::core::model::catalog::Error::UnknownId(_) => "notfound.element",
        },
        exit: Exit::Violation,
        message: format!("{path}: {failure}"),
    }
}

fn not_a_project(env: &Env<'_>) -> Error {
    Error {
        code: "env.not_a_project",
        exit: Exit::Env,
        message: format!("{} has no Cargo.toml", env.root),
    }
}
