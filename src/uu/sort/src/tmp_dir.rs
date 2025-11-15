// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use tempfile::TempDir;
use uucore::{
    error::{UResult, USimpleError},
    show_error, translate,
};

use crate::SortError;

/// A wrapper around [`TempDir`] that may only exist once in a process.
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

#[derive(Default, Clone)]
struct HandlerRegistration {
    lock: Option<Arc<Mutex<()>>>,
    path: Option<PathBuf>,
}

fn handler_state() -> Arc<Mutex<HandlerRegistration>> {
    // Lazily create the global HandlerRegistration so all TmpDirWrapper instances and the
    // SIGINT handler operate on the same lock/path snapshot.
    static HANDLER_STATE: OnceLock<Arc<Mutex<HandlerRegistration>>> = OnceLock::new();
    HANDLER_STATE
        .get_or_init(|| Arc::new(Mutex::new(HandlerRegistration::default())))
        .clone()
}

fn ensure_signal_handler_installed(state: Arc<Mutex<HandlerRegistration>>) -> UResult<()> {
    // This shared state must originate from `handler_state()` so the handler always sees
    // the current lock/path pair and can clean up the active temp directory on SIGINT.
    // Install a shared SIGINT handler so the active temp directory is deleted when the user aborts.
    // Guard to ensure the SIGINT handler is registered once per process and reused.
    static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

    if HANDLER_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    let handler_state = state.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        // Load the latest lock/path snapshot so the handler cleans the active temp dir.
        let (lock, path) = {
            let state = handler_state.lock().unwrap();
            (state.lock.clone(), state.path.clone())
        };

        if let Some(lock) = lock {
            let _guard = lock.lock().unwrap();
            if let Some(path) = path {
                if let Err(e) = remove_tmp_dir(&path) {
                    show_error!(
                        "{}",
                        translate!(
                            "sort-failed-to-delete-temporary-directory",
                            "error" => e
                        )
                    );
                }
            }
        }

        std::process::exit(2)
    }) {
        HANDLER_INSTALLED.store(false, Ordering::Release);
        return Err(USimpleError::new(
            2,
            translate!("sort-failed-to-set-up-signal-handler", "error" => e),
        ));
    }

    Ok(())
}

impl TmpDirWrapper {
    pub fn new(path: PathBuf) -> Self {
        Self {
            parent_path: path,
            size: 0,
            temp_dir: None,
            lock: Arc::default(),
        }
    }

    fn init_tmp_dir(&mut self) -> UResult<()> {
        assert!(self.temp_dir.is_none());
        assert_eq!(self.size, 0);
        self.temp_dir = Some(
            tempfile::Builder::new()
                .prefix("uutils_sort")
                .tempdir_in(&self.parent_path)
                .map_err(|_| SortError::TmpFileCreationFailed {
                    path: self.parent_path.clone(),
                })?,
        );

        let path = self.temp_dir.as_ref().unwrap().path().to_owned();
        let state = handler_state();
        {
            let mut guard = state.lock().unwrap();
            guard.lock = Some(self.lock.clone());
            guard.path = Some(path);
        }

        ensure_signal_handler_installed(state)
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

    /// Function just waits if signal handler was called
    pub fn wait_if_signal(&self) {
        let _lock = self.lock.lock().unwrap();
    }
}

impl Drop for TmpDirWrapper {
    fn drop(&mut self) {
        let state = handler_state();
        let mut guard = state.lock().unwrap();

        if guard
            .lock
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &self.lock))
        {
            guard.lock = None;
            guard.path = None;
        }
    }
}

/// Remove the directory at `path` by deleting its child files and then itself.
/// Errors while deleting child files are ignored.
fn remove_tmp_dir(path: &Path) -> std::io::Result<()> {
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for file in read_dir.flatten() {
            // if we fail to delete the file here it was probably deleted by another thread
            // in the meantime, but that's ok.
            let _ = std::fs::remove_file(file.path());
        }
    }
    std::fs::remove_dir(path)
}
