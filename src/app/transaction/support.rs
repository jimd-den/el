// Shared fixtures for the transaction tests: a committed fake crate, a fixed
// clock, and a `Tools` double that answers the lint gate without a process. They
// are support, not tests, so both test files can describe the same starting state.

use super::{apply, Mutation};
use std::cell::RefCell;
use std::collections::VecDeque;

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

pub(super) struct Scripted {
    pub(super) check: RefCell<VecDeque<String>>,
    pub(super) clippy: RefCell<VecDeque<String>>,
    pub(super) test: RefCell<VecDeque<String>>,
    pub(super) timed_out: bool,
}

impl Scripted {
    pub(super) fn clean() -> Self {
        Self {
            check: RefCell::new(VecDeque::new()),
            clippy: RefCell::new(VecDeque::new()),
            test: RefCell::new(VecDeque::new()),
            timed_out: false,
        }
    }

    pub(super) fn clippy_rejects() -> Self {
        let mut scripted = Self::clean();
        scripted.clippy = RefCell::new(VecDeque::from([CLIPPY_FRAME.to_string()]));
        scripted
    }

    pub(super) fn with_check(responses: Vec<String>) -> Self {
        let mut scripted = Self::clean();
        scripted.check = RefCell::new(VecDeque::from(responses));
        scripted
    }
}

/// One clippy diagnostic frame, enough for the gate to refuse on.
const CLIPPY_FRAME: &str = r#"{"reason":"compiler-message","message":{"level":"error","message":"unneeded `return` statement","code":{"code":"clippy::needless_return"},"spans":[{"file_name":"src/main.rs","line_start":2,"column_start":1,"is_primary":true}]}}"#;

fn answer(queue: &RefCell<VecDeque<String>>) -> Finished {
    Finished {
        code: 0,
        stdout: queue.borrow_mut().pop_front().unwrap_or_default(),
        stderr: String::new(),
        timed_out: false,
    }
}

impl Tools for Scripted {
    fn run(&self, argv: &[String], _stdin: Option<&str>) -> Result<Finished, ToolError> {
        let second = argv.get(1).map(String::as_str).unwrap_or("");
        Ok(match second {
            "check" => answer(&self.check),
            "clippy" => answer(&self.clippy),
            "test" => answer(&self.test),
            _ => Finished {
                code: 0,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
            },
        })
    }

    fn run_timed(
        &self,
        argv: &[String],
        stdin: Option<&str>,
        _secs: u64,
    ) -> Result<Finished, ToolError> {
        if self.timed_out {
            return Ok(Finished {
                code: -1,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            });
        }
        self.run(argv, stdin)
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
        fast: false,
    }
}

pub(super) fn run(memory: &Memory, store: &Store<'_>, clock: &Fixed, tools: &Scripted, mutation: Mutation) -> Report {
    let env = Env {
        fs: memory,
        journal: store,
        clock,
        tools,
        actor: "human".to_string(),
        crate_name: "demo".to_string(),
        root: ".".to_string(),
        cargo_timeout: 120,
    };
    apply(&env, mutation)
}
