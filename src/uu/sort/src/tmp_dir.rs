use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tempfile::TempDir;
use uucore::error::{UResult, USimpleError};

use crate::SortError;

/// A wrapper around TempDir that may only exist once in a process.
///
/// `TmpDirWrapper` handles the allocation of new temporary files in this temporary directory and
/// deleting the whole directory when `SIGINT` is received. Creating a second `TmpDirWrapper` will
/// fail because `ctrlc::set_handler()` fails when there's already a handler.
/// The directory is only created once the first file is requested.
pub struct TmpDirWrapper {
    temp_dir: Option<TempDir>,
    parent_path: PathBuf,
    size: usize,
    lock: Arc<Mutex<()>>,
}

impl TmpDirWrapper {
    pub fn new(path: PathBuf) -> Self {
        Self {
            parent_path: path,
            size: 0,
            temp_dir: None,
            lock: Default::default(),
        }
    }

    fn init_tmp_dir(&mut self) -> UResult<()> {
        assert!(self.temp_dir.is_none());
        assert_eq!(self.size, 0);
        self.temp_dir = Some(
            tempfile::Builder::new()
                .prefix("uutils_sort")
                .tempdir_in(&self.parent_path)
                .map_err(|_| SortError::TmpDirCreationFailed)?,
        );

        let path = self.temp_dir.as_ref().unwrap().path().to_owned();
        let lock = self.lock.clone();
        ctrlc::set_handler(move || {
            // Take the lock so that `next_file_path` returns no new file path.
            let _lock = lock.lock().unwrap();
            if let Err(e) = remove_tmp_dir(&path) {
                show_error!("failed to delete temporary directory: {}", e);
            }
            std::process::exit(2)
        })
        .map_err(|e| USimpleError::new(2, format!("failed to set up signal handler: {}", e)))
    }

    pub fn next_file(&mut self) -> UResult<(File, PathBuf)> {
        if self.temp_dir.is_none() {
            self.init_tmp_dir()?;
        }

        let _lock = self.lock.lock().unwrap();
        let file_name = self.size.to_string();
        self.size += 1;
        let path = self.temp_dir.as_ref().unwrap().path().join(file_name);
        Ok((
            File::create(&path).map_err(|error| SortError::OpenTmpFileFailed { error })?,
            path,
        ))
    }
}

/// Remove the directory at `path` by deleting its child files and then itself.
/// Errors while deleting child files are ignored.
fn remove_tmp_dir(path: &Path) -> std::io::Result<()> {
    if let Ok(read_dir) = std::fs::read_dir(&path) {
        for file in read_dir.flatten() {
            // if we fail to delete the file here it was probably deleted by another thread
            // in the meantime, but that's ok.
            let _ = std::fs::remove_file(file.path());
        }
    }
    std::fs::remove_dir(path)
}
