// Running a program, with `std::process`. The whole of `ports::Tools`, at the one
// place the operating system is allowed to be.
//
// Captured output, and stdin written before the wait — which is safe for the only
// caller that passes stdin (`--body -` reads it before calling anything, so `el`
// never deadlocks on a child waiting for a pipe it is not writing yet).
//
// It deliberately does not set timeouts. A timeout needs a thread or a signal, and
// the tiers that need one (rust-analyzer, M10) will need a different shape than a
// one-shot `Command::new(..).output()`.

use crate::ports::tools::{Error, Finished, Tools};
use std::io::Write;
use std::process::{Command, Stdio};

/// The real subprocess runner.
pub struct ProcTools;

impl Tools for ProcTools {
    fn run(&self, argv: &[String], stdin: Option<&str>) -> Result<Finished, Error> {
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| Error {
                program: String::new(),
                missing: false,
                detail: "nothing to run".to_string(),
            })?;

        let mut child = Command::new(program)
            .args(args)
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => Error::missing(program),
                _ => Error {
                    program: program.clone(),
                    missing: false,
                    detail: error.to_string(),
                },
            })?;

        if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
            // A child that exits before reading all of stdin makes this fail with
            // EPIPE; that is the child's verdict, reported through its exit code.
            let _ = pipe.write_all(text.as_bytes());
        }

        let output = child.wait_with_output().map_err(|error| Error {
            program: program.clone(),
            missing: false,
            detail: error.to_string(),
        })?;

        Ok(Finished {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
