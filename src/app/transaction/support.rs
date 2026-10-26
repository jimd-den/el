// Shared fixtures for the transaction tests: a committed fake crate, a fixed
// clock, and a `Tools` double that answers the lint gate without a process. They
// are support, not tests, so both test files can describe the same starting state.

use super::{apply, Mutation};
use crate::app::report::Report;
use crate::app::Env;
use crate::core::history::entry::{Entry, Op};
use crate::core::history::tree::Tree;
use crate::core::model::body::Body;
use crate::core::model::catalog::Catalog;
use crate::core::model::change::Change;
use crate::core::model::element::{BodyMode, NewElement, Visibility};
use crate::core::model::kind::Kind;
use crate::core::render::render;
use crate::adapters::store::Store;
use crate::ports::clock::Clock;
use crate::ports::fs_fake::Memory;
use crate::ports::tools::{Error as ToolError, Finished, Tools};
use crate::ports::Fs;

pub(super) struct Fixed(pub(super) i64);

impl Clock for Fixed {
    fn now_unix(&self) -> i64 {
        self.0
    }
}

pub(super) struct Lint(pub(super) bool);

impl Tools for Lint {
    fn run(&self, _argv: &[String], _stdin: Option<&str>) -> Result<Finished, ToolError> {
        Ok(Finished {
            code: if self.0 { 0 } else { 1 },
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

pub(super) fn catalog() -> Catalog {
    let mut catalog = Catalog::new();
    catalog
        .add_element(&NewElement {
            kind: Kind::Function,
            module: "src/main.rs",
            signature: "fn run()",
            visibility: Visibility::Private,
        })
        .expect("add");
    catalog
}

pub(super) fn seed(memory: &Memory, store: &Store<'_>, catalog: &Catalog) {
    memory
        .write("el.policy", b"require_preamble = false\n")
        .expect("policy");
    store.stamp_version().expect("version");
    store.set_catalog(&catalog.encode()).expect("catalog");

    let projection = render(catalog, "demo");
    let mut entries = Vec::new();
    for file in &projection.files {
        let digest = store.put_blob(file.text.as_bytes()).expect("blob");
        memory.write(&file.path, file.text.as_bytes()).expect("projection");
        entries.push((file.path.clone(), digest));
    }
    let tree = Tree::new(entries).expect("tree");
    let to = store.put_tree(&tree).expect("put tree");
    let empty = Tree::new(Vec::new()).expect("empty");
    let from = store.put_tree(&empty).expect("empty tree");
    store
        .put_entry(&Entry {
            seq: 1,
            ts: 0,
            actor: "human".to_string(),
            op: Op::Init,
            target: "-".to_string(),
            from,
            to,
            gate: "model".to_string(),
            msg: "seed".to_string(),
        })
        .expect("entry");
    store.set_head(1).expect("head");
}

pub(super) fn add() -> Mutation {
    Mutation {
        cmd: "add",
        op: Op::Add,
        target: "fn added".to_string(),
        message: "add fn added".to_string(),
        changes: vec![Change::AddElement {
            kind: Kind::Function,
            module: "src/main.rs".to_string(),
            signature: "fn added()".to_string(),
            visibility: Visibility::Private,
            body: Body::default(),
        mode: BodyMode::Split,
        }],
        dry_run: false,
        strict: false,
    }
}

pub(super) fn run(memory: &Memory, store: &Store<'_>, clock: &Fixed, tools: &Lint, mutation: Mutation) -> Report {
    let env = Env {
        fs: memory,
        journal: store,
        clock,
        tools,
        actor: "human".to_string(),
        crate_name: "demo".to_string(),
        root: ".".to_string(),
    };
    apply(&env, mutation)
}
