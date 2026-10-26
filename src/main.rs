// The composition root. It is the only file that names a concrete adapter, reads
// the environment, or exits the process. Everything below it returns values.
//
// Order matters: parse before touching a crate (a usage error writes nothing),
// acquire the lock before a mutating command, build the ports once, then hand the
// parsed command to `app`. The renderer is chosen last, from the same `Report`.

use el::adapters::{Local, Store, SystemClock, ProcTools};
use el::adapters::lock::Lock;
use el::app::{self, Env};
use el::cli::{self, Command, Source};
use el::ports::Clock;
use std::io::Read;
use std::path::Path;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match cli::parse(&argv) {
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            std::process::exit(1);
        }
    };

    let root = match &parsed.command {
        Command::New { path, .. } => path.clone(),
        _ => parsed.globals.root.clone().unwrap_or_else(|| ".".to_string()),
    };
    let crate_name = Path::new(&root)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "crate".to_string());
    let preview = matches!(&parsed.command, Command::New { .. }) && parsed.globals.dry_run;
    if root != "." && !preview {
        if matches!(parsed.command, Command::New { .. })
            && let Err(error) = std::fs::create_dir_all(&root)
        {
            eprintln!("el: {root}: {error}");
            std::process::exit(4);
        }
        if let Err(error) = std::env::set_current_dir(&root) {
            eprintln!("el: {root}: {error}");
            std::process::exit(4);
        }
    }

    let local = Local::new(".");
    let store = Store::new(&local);
    let clock = SystemClock;
    let tools = ProcTools;
    let actor = std::env::var("EL_ACTOR").unwrap_or_else(|_| "human".to_string());
    let cargo_timeout = std::env::var("EL_CARGO_TIMEOUT")
        .ok()
        .and_then(|text| text.parse::<u64>().ok())
        .unwrap_or(120);
    let env = Env {
        fs: &local,
        journal: &store,
        clock: &clock,
        tools: &tools,
        actor,
        crate_name,
        root,
        cargo_timeout,
    };

    let _lock = if needs_lock(&parsed.command) {
        match Lock::acquire(Path::new("."), clock.now_unix() as u64) {
            Ok(lock) => Some(lock),
            Err(held) => {
                eprintln!("{held}");
                std::process::exit(5);
            }
        }
    } else {
        None
    };

    let stdin = if uses_stdin(&parsed.command) {
        let mut text = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut text) {
            eprintln!("el: could not read stdin: {error}");
            std::process::exit(4);
        }
        text
    } else {
        String::new()
    };

    let report = app::run(parsed.clone(), &env, &stdin);
    if parsed.globals.json {
        print!("{}", el::adapters::out_json::render(&report));
    } else {
        print!("{}", el::adapters::out_human::stdout(&report));
        eprint!("{}", el::adapters::out_human::stderr(&report, parsed.globals.quiet));
    }
    std::process::exit(report.exit().code() as i32);
}

fn needs_lock(command: &Command) -> bool {
    matches!(
        command,
        Command::New { .. }
            | Command::Init { .. }
            | Command::Add(_)
            | Command::Edit(_)
            | Command::Rm { .. }
            | Command::Mv(_)
            | Command::Run { .. }
            | Command::Undo { .. }
            | Command::Redo
            | Command::Revert { .. }
            | Command::Doctor { .. }
    )
}

fn uses_stdin(command: &Command) -> bool {
    fn source(source: &Option<Source>) -> bool {
        matches!(source, Some(Source::Stdin))
    }
    match command {
        Command::Add(add) => source(&add.body) || source(&add.preamble) || source(&add.doc),
        Command::Edit(edit) => source(&edit.body) || source(&edit.doc),
        _ => false,
    }
}
