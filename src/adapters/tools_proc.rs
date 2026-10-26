// Running a program, with `std::process`. The whole of `ports::Tools`, at the one
// place the operating system is allowed to be.
//
// Captured output, and stdin written before the wait — which is safe for the only
// caller that passes stdin (`--body -` reads it before calling anything, so `el`
// never deadlocks on a child waiting for a pipe it is not writing yet).
//
// `run_timed` drains both pipes on reader threads and polls the child, so a
// runaway tool can be killed at the deadline. On timeout the reader threads are
// detached rather than joined: a child that left descendants holding its pipes
// open would otherwise block the join forever, and the process exits right after
// the gate reports.

use crate::ports::tools::{Error, Finished, Tools};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The real subprocess runner.
pub struct ProcTools;

fn spawn(argv: &[String], stdin: Option<&str>) -> Result<Child, Error> {
    let (program, args) = argv.split_first().ok_or_else(|| Error {
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
    Ok(child)
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    })
}

impl Tools for ProcTools {
    fn run(&self, argv: &[String], stdin: Option<&str>) -> Result<Finished, Error> {
        let child = spawn(argv, stdin)?;
        let program = argv.first().cloned().unwrap_or_default();
        let output = child.wait_with_output().map_err(|error| Error {
            program,
            missing: false,
            detail: error.to_string(),
        })?;

        Ok(Finished {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            timed_out: false,
        })
    }

    fn run_timed(
        &self,
        argv: &[String],
        stdin: Option<&str>,
        timeout_secs: u64,
    ) -> Result<Finished, Error> {
        let mut child = spawn(argv, stdin)?;
        let program = argv.first().cloned().unwrap_or_default();
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Ok(Finished {
                        code: status.code().unwrap_or(-1),
                        stdout: stdout.join().unwrap_or_default(),
                        stderr: stderr.join().unwrap_or_default(),
                        timed_out: false,
                    });
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(Finished {
                            code: -1,
                            stdout: String::new(),
                            stderr: String::new(),
                            timed_out: true,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    return Err(Error {
                        program,
                        missing: false,
                        detail: error.to_string(),
                    });
                }
            }
        }
    }
}
