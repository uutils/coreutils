// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
use signal_hook::{consts::SIGINT, iterator::Signals};
#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
use std::path::Path;
#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};

use tempfile::TempDir;
use uucore::error::UResult;

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

// Lazily create the global HandlerRegistration so all TmpDirWrapper instances and the
// SIGINT handler operate on the same lock/path snapshot.
static HANDLER_STATE: LazyLock<Arc<Mutex<HandlerRegistration>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HandlerRegistration::default())));

#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
fn ensure_signal_handler_installed(state: Arc<Mutex<HandlerRegistration>>) {
    // This shared state must originate from `HANDLER_STATE` so the handler always sees
    // the current lock/path pair and can clean up the active temp directory on SIGINT.
    // Install a shared SIGINT handler so the active temp directory is deleted when the user aborts.
    // Guard to ensure the SIGINT handler is registered once per process and reused.
    static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

    if HANDLER_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    std::thread::spawn(move || {
        // silently ignore errors since cleaning up temporary files is not a serious task
        let Ok(mut signals) = Signals::new([SIGINT]) else {
            return;
        };

        for _ in signals.forever() {
            let Ok(state) = state.lock() else {
                return;
            };
            if let (Some(lock), Some(path)) = (state.lock.clone(), state.path.clone())
                && lock.lock().is_ok()
            {
                let _ = remove_tmp_dir(&path);
            }
            // signal_hook::low_level::raise(SIGINT) is not required
        }
    });
}

#[cfg(any(target_os = "redox", target_os = "wasi"))]
#[allow(clippy::unnecessary_wraps)]
fn ensure_signal_handler_installed(_state: Arc<Mutex<HandlerRegistration>>) -> UResult<()> {
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
        let state = HANDLER_STATE.clone();
        {
            let mut guard = state.lock().unwrap();
            guard.lock = Some(self.lock.clone());
            guard.path = Some(path);
        }

        // Always attempt to install the signal handler so that Ctrl+C
        // triggers cleanup. Failure is non-fatal: sort still works,
        // just without SIGINT-triggered temp directory removal.
        ensure_signal_handler_installed(state);
        Ok(())
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
        let state = HANDLER_STATE.clone();
        let mut guard = state.lock().unwrap();

        if guard
            .lock
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &self.lock))
        {
            guard.lock = None;
            guard.path = None;
        }
        drop(guard);

        // Explicitly attempt cleanup before TempDir's Drop runs silently.
        // TempDir::drop uses `let _ = remove_dir_all()` which silently
        // ignores errors, potentially leaking the directory.
        #[cfg(not(any(target_os = "redox", target_os = "wasi")))]
        if let Some(ref temp_dir) = self.temp_dir {
            let _ = remove_tmp_dir(temp_dir.path());
        }
    }
}

/// Remove the directory at `path` by deleting its child files and then itself.
/// Errors while deleting child files are ignored.
#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
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
