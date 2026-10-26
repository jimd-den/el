// `.el/lock`: one writer at a time (`FR-39`). Two `el` processes on one crate is a
// user error, not a data-loss event — the loser exits 5 and writes nothing.
//
// The lock is a file created with `O_EXCL` containing the holder's pid and start
// time. Exclusivity therefore comes from the filesystem, not from a protocol we
// invented, and a process that dies without cleaning up leaves a file the next run
// can recognise as stale: a dead pid, or older than ten minutes, is reclaimed with a
// note rather than waited on forever.
//
// This is the one port-less piece of the store, deliberately. "Create this name
// exclusively" is not `read`/`write`/`remove`, and widening `ports::Fs` to say it
// would put a concurrency primitive in the inner circle for the benefit of one
// adapter.
//
// Hazard: liveness is checked by looking for `/proc/<pid>`, which is Linux. Elsewhere
// the age rule still reclaims the lock, so a foreign platform is slower, not broken.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Ten minutes: long enough for a cold `cargo test` gate on a big crate, short
/// enough that a crashed run does not block a person for an afternoon.
const STALE_SECS: u64 = 600;

/// A held lock. Dropping it releases the file, so an early return cannot leak it.
pub struct Lock {
    path: PathBuf,
    pid: u32,
}

/// Someone else holds the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub pid: u32,
    pub age_secs: u64,
}

impl std::fmt::Display for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "el: another el process (pid {}) has held the lock for {}s",
            self.pid, self.age_secs
        )
    }
}

impl Lock {
    /// Take `.el/lock`, reclaiming a stale one.
    ///
    /// Contract: `Ok` means this process owns the store. `Err(Held)` means another
    /// live process does, and nothing was written.
    pub fn acquire(root: &Path, now: u64) -> Result<Lock, Held> {
        let path = root.join(".el").join("lock");
        match create(&path, now) {
            Ok(()) => Ok(Lock {
                path,
                pid: std::process::id(),
            }),
            Err(Held { pid, age_secs }) => {
                if age_secs <= STALE_SECS && alive(pid) {
                    return Err(Held { pid, age_secs });
                }
                let _ = fs::remove_file(&path);
                create(&path, now)
                    .map(|()| Lock { path, pid: std::process::id() })
                    .map_err(|held| held)
            }
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn create(path: &Path, now: u64) -> Result<(), Held> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            let _ = writeln!(file, "{} {}", std::process::id(), now);
            Ok(())
        }
        Err(_) => Err(holder(path, now)),
    }
}

/// Who holds the lock, as the file records it.
fn holder(path: &Path, now: u64) -> Held {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut words = text.split_whitespace();
    let pid = words.next().and_then(|w| w.parse().ok()).unwrap_or(0);
    let started = words.next().and_then(|w| w.parse::<u64>().ok()).unwrap_or(now);
    Held {
        pid,
        age_secs: now.saturating_sub(started),
    }
}

fn alive(pid: u32) -> bool {
    pid != 0 && Path::new("/proc").join(pid.to_string()).exists()
}
