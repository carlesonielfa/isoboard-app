//! Original capture images kept beside the history database.
//!
//! Each kept original is one owner-only file named by its entry id. The
//! database decides which originals exist (`original_bytes IS NOT NULL`);
//! this directory only mirrors that set. Writes go through a temporary file
//! and a rename, and `retain_only` removes every file the database no longer
//! lists, so a failed transaction, a prune, a delete or a crash between the
//! two steps all end the same way once the next sweep runs.

use std::collections::HashSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::HistoryError;

const TEMPORARY_SUFFIX: &str = ".partial";

pub(super) struct OriginalsDirectory {
    path: PathBuf,
}

impl OriginalsDirectory {
    pub(super) fn open(path: PathBuf) -> Result<Self, HistoryError> {
        fs::create_dir_all(&path).map_err(|error| io_error(&path, error))?;
        restrict(&path, 0o700)?;
        Ok(Self { path })
    }

    pub(super) fn write(&self, id: &str, bytes: &[u8]) -> Result<(), HistoryError> {
        if id.is_empty()
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(HistoryError::InvalidEntry(
                "history entry id cannot name an original".into(),
            ));
        }
        let destination = self.file(id);
        let temporary = self.path.join(format!("{id}{TEMPORARY_SUFFIX}"));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|error| io_error(&temporary, error))?;
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| io_error(&temporary, error))?;
            fs::rename(&temporary, &destination).map_err(|error| io_error(&destination, error))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    #[cfg(test)]
    pub(super) fn read(&self, id: &str) -> Result<Vec<u8>, HistoryError> {
        let path = self.file(id);
        fs::read(&path).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => HistoryError::NotFound,
            _ => io_error(&path, error),
        })
    }

    /// Removes every file whose entry id is not in `kept`, including
    /// temporary files left by an interrupted write.
    pub(super) fn retain_only(&self, kept: &HashSet<String>) -> Result<(), HistoryError> {
        let entries = fs::read_dir(&self.path).map_err(|error| io_error(&self.path, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| io_error(&self.path, error))?;
            let name = entry.file_name();
            if name.to_str().is_some_and(|name| kept.contains(name)) {
                continue;
            }
            let path = entry.path();
            if let Err(error) = fs::remove_file(&path) {
                eprintln!(
                    "isoboard: could not remove history original {}: {error}",
                    path.display()
                );
            }
        }
        Ok(())
    }

    fn file(&self, id: &str) -> PathBuf {
        self.path.join(id)
    }
}

fn restrict(path: &Path, mode: u32) -> Result<(), HistoryError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|error| io_error(path, error))?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn io_error(path: &Path, error: std::io::Error) -> HistoryError {
    HistoryError::Io(format!("{}: {error}", path.display()))
}
