// The local filesystem, behind `ports::Fs`. Every fact about *how* a disk behaves
// lives here: temporary files, `fsync`, `rename`, and which errors are which.
//
// Shape: one root directory, store-relative paths, and a write that stages into
// `tmp/`, syncs, then renames. Ordering is the whole point (`HISTORY.md` §7):
// a rename is atomic on POSIX, so a crash before it leaves the old file and a
// crash after it leaves the new one. There is no third outcome to report.
//
// It deliberately does not lock, detect drift, or hold a transaction: those arrive
// with the first verb and need a process identity this module has no way to invent;
// they live in `adapters::lock` and `app::projection` instead. `tmp/` is left in place
// rather than emptied at construction, so a test can
// show that a leftover staging file is never trusted — the sweep belongs to startup.

use crate::ports::fs::{Error, Fs, Kind};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A store root on the real filesystem.
pub struct Local {
    root: PathBuf,
    /// Distinguishes two staged writes of one path inside one process.
    staged: AtomicU64,
}

impl Local {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            staged: AtomicU64::new(0),
        }
    }

    /// Where this store lives. Used by the tests that must look at the disk itself.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The absolute path for a store path, refusing anything that could leave the
    /// root (`FR-33`'s sibling: a path read from a record is untrusted input).
    fn resolve(&self, path: &str) -> Result<PathBuf, Error> {
        let relative = Path::new(path);
        let escapes = relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)));
        if escapes || path.is_empty() {
            return Err(Error::new(path, Kind::NotAFile));
        }
        Ok(self.root.join(relative))
    }

    /// A fresh path under `tmp/` for the bytes about to replace `target`.
    fn staging_path(&self, target: &Path) -> PathBuf {
        let count = self.staged.fetch_add(1, Ordering::Relaxed);
        let name = target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "blob".to_string());
        self.root
            .join("tmp")
            .join(format!("{name}.{}.{count}.tmp", std::process::id()))
    }
}

impl Fs for Local {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        match fs::read(self.resolve(path)?) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Error::new(path, Kind::from_io(error.kind()))),
        }
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Error> {
        let target = self.resolve(path)?;
        let parent = target.parent().unwrap_or(&self.root);
        fs::create_dir_all(parent).map_err(|e| Error::new(path, Kind::from_io(e.kind())))?;
        fs::create_dir_all(self.root.join("tmp"))
            .map_err(|e| Error::new(path, Kind::from_io(e.kind())))?;

        let staged = self.staging_path(&target);
        stage(&staged, bytes, path)?;
        fs::rename(&staged, &target).map_err(|e| Error::new(path, Kind::from_io(e.kind())))?;
        sync_directory(parent, path)
    }

    /// A path the user named is theirs, not the store's: `resolve` exists to keep
    /// the projection inside the crate, and this method only ever reads. A
    /// relative path is joined to the root without the `..` refusal, because
    /// `../snippet.rs` is how a sibling is named.
    fn read_source(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        if path.is_empty() {
            return Err(Error::new(path, Kind::NotAFile));
        }
        let target = Path::new(path);
        let target = if target.is_absolute() {
            target.to_path_buf()
        } else {
            self.root.join(target)
        };
        match fs::read(&target) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Error::new(path, Kind::from_io(error.kind()))),
        }
    }

    fn remove(&self, path: &str) -> Result<(), Error> {
        match fs::remove_file(self.resolve(path)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Error::new(path, Kind::from_io(error.kind()))),
        }
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, Error> {
        let mut found = Vec::new();
        let base = if prefix.is_empty() {
            self.root.clone()
        } else {
            self.resolve(prefix)?
        };
        walk(&self.root, &base, &mut found)?;
        found.sort();
        Ok(found)
    }

    fn exists(&self, path: &str) -> Result<bool, Error> {
        Ok(self.resolve(path)?.is_file())
    }
}

/// Write `bytes` to `staged` and make them durable before anyone renames them.
fn stage(staged: &Path, bytes: &[u8], path: &str) -> Result<(), Error> {
    let mut file = File::create(staged).map_err(|e| Error::new(path, Kind::from_io(e.kind())))?;
    file.write_all(bytes)
        .map_err(|e| Error::new(path, Kind::from_io(e.kind())))?;
    file.sync_all()
        .map_err(|e| Error::new(path, Kind::from_io(e.kind())))
}

/// Make a rename durable: without this the file's name can be lost on power loss
/// even though its contents were synced.
fn sync_directory(directory: &Path, path: &str) -> Result<(), Error> {
    File::open(directory)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| Error::new(path, Kind::from_io(e.kind())))
}

/// Every file under `base`, as store-relative paths. `tmp/` is not part of a store.
fn walk(root: &Path, base: &Path, found: &mut Vec<String>) -> Result<(), Error> {
    let listing = match fs::read_dir(base) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Error::new(
                base.display().to_string(),
                Kind::from_io(error.kind()),
            ));
        }
    };
    for entry in listing {
        let entry =
            entry.map_err(|e| Error::new(base.display().to_string(), Kind::from_io(e.kind())))?;
        let path = entry.path();
        let relative = match path.strip_prefix(root) {
            Ok(relative) => relative.to_string_lossy().into_owned(),
            Err(_) => continue,
        };
        // Staging is not store content: a leftover tmp file is never a blob.
        if relative == "tmp" || relative.starts_with("tmp/") {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, found)?;
        } else {
            found.push(relative);
        }
    }
    Ok(())
}
