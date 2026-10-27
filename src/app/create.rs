// `el new`: the one command that creates the store instead of reading it. It
// writes the files a crate needs, seeds history with an `init` entry, then adds
// the root module as the first ordinary transaction — which is why `el log` shows
// two entries before the author types anything.
//
// It deliberately does not call `cargo new`: the projection is ours to write, and
// a dependency on cargo's layout would put a second source of truth in the tree.

use super::error;
use super::report::Report;
use super::transaction::Mutation;
use super::Env;
use crate::cli::request::Globals;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::tree::Tree;
use crate::core::model::body::{Body, BodyStmt};
use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::element::{BodyMode, Visibility};
use crate::core::model::kind::Kind;

pub(crate) const POLICY: &str = "ceiling = 200\nmax_fn_lines = 15\nrequire_preamble = true\nforbid = unsafe\ndeps = none\ngate = check\n";

pub fn new(env: &Env<'_>, globals: &Globals, path: &str, lib: bool, _policy: bool) -> Report {
    if globals.dry_run {
        return Report::fail("new", error::unsupported("new", "--dry-run"));
    }
    if env.fs.exists("Cargo.toml").unwrap_or(false) || env.fs.exists(".el/version").unwrap_or(false) {
        return Report::fail(
            "new",
            error::Error {
                code: "usage.exists",
                exit: super::Exit::Usage,
                message: format!("{path} already holds a crate"),
            },
        );
    }
    if let Err(error) = seed(env) {
        return Report::fail("new", error);
    }
    let manifest = crate::core::render::manifest::render(&env.crate_name);
    if let Err(error) = env.fs.write("Cargo.toml", manifest.as_bytes()) {
        return Report::fail("new", error::filesystem(error));
    }

    let root = if lib { "src/lib.rs" } else { "src/main.rs" };
    let keyword = if lib { "lib" } else { "main" };
    let preamble = format!("// {} — created by el; change it with `el edit`.", env.crate_name);
    let change = Change::AddElement {
        kind: Kind::Module,
        module: root.to_string(),
        signature: format!("mod {keyword}"),
        visibility: Visibility::Private,
        body: Body {
            statements: vec![BodyStmt {
                text: preamble,
                children: Vec::new(),
            }],
        },
        mode: BodyMode::Split,
    };
    let mut report = super::transaction::apply(
        env,
        Mutation {
            cmd: "new",
            op: Op::Add,
            target: "-".to_string(),
            message: format!("created {}", env.crate_name),
            changes: vec![change],
            origins: Vec::new(),
            dry_run: false,
            strict: globals.strict,
            // The scaffold is not a user change: it always runs at L0, and
            // the user's tier applies from their first real change.
            fast: true,
        },
    );
    if report.error.is_none() {
        report.lines = vec![format!("created {}", env.root)];
    }
    report
}

fn seed(env: &Env<'_>) -> Result<(), error::Error> {
    env.fs.write("el.policy", POLICY.as_bytes()).map_err(error::filesystem)?;
    env.fs.write(".gitignore", b".el/\ntarget/\ntmp/\n").map_err(error::filesystem)?;
    env.journal.stamp_version().map_err(error::store)?;

    let empty = Tree::new(Vec::new()).expect("an empty tree");
    let digest = env.journal.put_tree(&empty).map_err(error::store)?;
    let entry = Entry {
        seq: 1,
        ts: env.clock.now_unix(),
        actor: env.actor.clone(),
        op: Op::Init,
        target: "-".to_string(),
        from: digest,
        to: digest,
        gate: "model".to_string(),
        msg: format!("created {}", env.crate_name),
    };
    env.journal.put_entry(&entry).map_err(error::store)?;
    env.journal.set_head(1).map_err(error::store)?;
    let catalog = Catalog::new().encode();
    env.journal.set_catalog(&catalog).map_err(error::store)?;
    env.journal.put_catalog(1, &catalog).map_err(error::store)?;
    Ok(())
}
