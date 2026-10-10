// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore CLOEXEC NONBLOCK RDONLY RUSR unlinkat WRONLY WUSR

#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
use std::sync::atomic::AtomicBool;
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, atomic::Ordering},
};
#[cfg(unix)]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

use uucore::error::UResult;
#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
use uucore::{error::USimpleError, show_error, translate};

use crate::SortError;

/// A wrapper around [`tempfile::TempDir`] that may only exist once in a process.
///
/// `TmpDirWrapper` handles the allocation of new temporary files in this temporary directory and
/// deleting the whole directory when `SIGINT` is received. Creating a second `TmpDirWrapper` will
/// fail because `ctrlc::set_handler()` fails when there's already a handler.
/// The directory is only created once the first file is requested.
pub struct TmpDirWrapper {
    dir: Option<Arc<SortTmpDir>>,
    parent_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

#[derive(Default, Clone)]
struct HandlerRegistration {
    lock: Option<Arc<Mutex<()>>>,
    dir: Option<Arc<SortTmpDir>>,
}

// Lazily create the global HandlerRegistration so all TmpDirWrapper instances and the
// SIGINT handler operate on the same lock/directory snapshot.
static HANDLER_STATE: LazyLock<Arc<Mutex<HandlerRegistration>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HandlerRegistration::default())));

#[cfg(not(any(target_os = "redox", target_os = "wasi")))]
fn ensure_signal_handler_installed(state: Arc<Mutex<HandlerRegistration>>) -> UResult<()> {
    // This shared state must originate from `HANDLER_STATE` so the handler always sees
    // the current lock/directory pair and can clean up the active temp directory on SIGINT.
    // Install a shared SIGINT handler so the active temp directory is deleted when the user aborts.
    // Guard to ensure the SIGINT handler is registered once per process and reused.
    static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

    if HANDLER_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    if let Err(e) = ctrlc::set_handler(move || {
        // Load the latest lock/directory snapshot so the handler cleans the active temp dir.
        let (lock, dir) = {
            let state = state.lock().unwrap();
            (state.lock.clone(), state.dir.clone())
        };

        if let Some(lock) = lock {
            let _guard = lock.lock().unwrap();
            if let Some(dir) = dir
                && let Err(e) = dir.remove()
            {
                show_error!(
                    "{}",
                    translate!(
                        "sort-failed-to-delete-temporary-directory",
                        "error" => e
                    )
                );
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

#[cfg(any(target_os = "redox", target_os = "wasi"))]
#[allow(clippy::unnecessary_wraps)]
fn ensure_signal_handler_installed(_state: Arc<Mutex<HandlerRegistration>>) -> UResult<()> {
    Ok(())
}

impl TmpDirWrapper {
    pub fn new(path: PathBuf) -> Self {
        Self {
            parent_path: path,
            dir: None,
            lock: Arc::default(),
        }
    }

    fn init_tmp_dir(&mut self) -> UResult<()> {
        assert!(self.dir.is_none());
        let mut builder = tempfile::Builder::new();
        builder.prefix("uutils_sort");
        // The chunks hold a copy of the input, so keep them out of reach of other
        // users instead of relying on whatever umask the process inherited.
        #[cfg(unix)]
        builder.permissions(Permissions::from_mode(0o700));
        let dir = SortTmpDir::new(&mut builder, &self.parent_path).map_err(|_| {
            SortError::TmpFileCreationFailed {
                path: self.parent_path.clone(),
            }
        })?;
        let dir = Arc::new(dir);
        self.dir = Some(Arc::clone(&dir));

        let state = HANDLER_STATE.clone();
        {
            let mut guard = state.lock().unwrap();
            guard.lock = Some(self.lock.clone());
            guard.dir = Some(dir);
        }

        // Always attempt to install the signal handler so that Ctrl+C
        // triggers cleanup. Failure is non-fatal: sort still works,
        // just without SIGINT-triggered temp directory removal.
        let _ = ensure_signal_handler_installed(state);
        Ok(())
    }

    pub fn next_file(&mut self) -> UResult<(File, TmpPath)> {
        if self.dir.is_none() {
            self.init_tmp_dir()?;
        }
        let dir = self.dir.as_ref().unwrap();

        let _lock = self.lock.lock().unwrap();
        // Count the name first, so cleanup removes the file even if creating it
        // fails after the file exists.
        let index = dir.created.fetch_add(1, Ordering::Relaxed);
        dir.create(index)
            .map_err(|error| SortError::OpenTmpFileFailed { error }.into())
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
            guard.dir = None;
        }
        drop(guard);

        #[cfg(not(any(target_os = "redox", target_os = "wasi")))]
        if let Some(dir) = &self.dir {
            let _ = dir.remove();
        }
    }
}

/// The directory created by [`TmpDirWrapper`], whose files are named `0`, `1`, ...
/// It is only used through a descriptor checked to be this directory, so once its
/// path leads elsewhere, sort neither reads nor removes anything there.
#[cfg(all(unix, not(target_os = "redox")))]
mod imp {
    use rustix::fs::{AtFlags, CWD, Mode, OFlags, openat, unlinkat};
    use std::fs::File;
    use std::io;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::{Builder, TempDir};

    /// The device and inode of a file.
    type FileId = (u64, u64);

    fn file_id(file: &File) -> io::Result<FileId> {
        let metadata = file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }

    fn open_dir(path: &Path) -> io::Result<File> {
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        Ok(openat(CWD, path, flags, Mode::empty())?.into())
    }

    pub struct SortTmpDir {
        temp_dir: TempDir,
        id: FileId,
        pub created: AtomicUsize,
    }

    impl SortTmpDir {
        /// Create the directory in `parent`.
        pub fn new(builder: &mut Builder<'_, '_>, parent: &Path) -> io::Result<Self> {
            // `remove` cleans up through the descriptor: `TempDir` would remove
            // the directory recursively by path, wherever that leads by then.
            let temp_dir = builder.disable_cleanup(true).tempdir_in(parent)?;
            // The path is resolved once more right after creating the directory;
            // there is no way to create and open a directory in one step.
            let id = open_dir(temp_dir.path())
                .and_then(|dir| file_id(&dir))
                .inspect_err(|_| {
                    let _ = std::fs::remove_dir(temp_dir.path());
                })?;
            Ok(Self {
                temp_dir,
                id,
                created: AtomicUsize::new(0),
            })
        }

        /// Open the directory, if its path still leads to it. The descriptor is
        /// only kept for a moment: sort may have none to spare.
        fn open_dir(&self) -> io::Result<File> {
            let dir = open_dir(self.temp_dir.path())?;
            if file_id(&dir)? != self.id {
                return Err(io::ErrorKind::NotFound.into());
            }
            Ok(dir)
        }

        /// Create the file `index`, which must not exist yet.
        pub fn create(self: &Arc<Self>, index: usize) -> io::Result<(File, TmpPath)> {
            let name = index.to_string();
            // Restrict the chunks too, so a directory whose mode is later relaxed
            // doesn't expose them.
            let file = File::from(openat(
                &self.open_dir()?,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )?);
            let tmp_path = TmpPath {
                dir: Arc::clone(self),
                path: self.temp_dir.path().join(name),
                index,
                id: file_id(&file)?,
            };
            Ok((file, tmp_path))
        }

        /// Remove the file `index`, ignoring failures.
        fn remove_file(&self, index: usize) {
            if let Ok(dir) = self.open_dir() {
                let _ = unlinkat(&dir, index.to_string().as_str(), AtFlags::empty());
            }
        }

        /// Remove the files created in the directory, then the directory itself.
        /// Errors while deleting the files are ignored.
        pub fn remove(&self) -> io::Result<()> {
            let dir = self.open_dir()?;
            for index in 0..self.created.load(Ordering::Relaxed) {
                let _ = unlinkat(&dir, index.to_string().as_str(), AtFlags::empty());
            }
            // Only removes an empty directory, and doesn't follow a symlink.
            std::fs::remove_dir(self.temp_dir.path())
        }
    }

    /// A file created by [`super::TmpDirWrapper::next_file`].
    pub struct TmpPath {
        dir: Arc<SortTmpDir>,
        path: PathBuf,
        index: usize,
        id: FileId,
    }

    impl TmpPath {
        pub fn path(&self) -> &Path {
            &self.path
        }

        /// Open the file for reading, if it is still the file created. This takes
        /// no descriptor of the directory, as merges reopen files while others are
        /// open.
        pub fn open(&self) -> io::Result<File> {
            // The path may lead elsewhere by now: don't wait for a FIFO there.
            let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
            let file = File::from(openat(CWD, &self.path, flags, Mode::empty())?);
            if file_id(&file)? != self.id {
                return Err(io::ErrorKind::NotFound.into());
            }
            Ok(file)
        }

        /// Delete the file, ignoring failures.
        pub fn remove(self) {
            self.dir.remove_file(self.index);
        }
    }
}

/// The directory created by [`TmpDirWrapper`], whose files are named `0`, `1`, ...
#[cfg(not(all(unix, not(target_os = "redox"))))]
mod imp {
    use std::fs::{File, OpenOptions};
    use std::io;
    #[cfg(target_os = "redox")]
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicUsize;
    #[cfg(not(any(target_os = "redox", target_os = "wasi")))]
    use std::sync::atomic::Ordering;
    use tempfile::{Builder, TempDir};

    pub struct SortTmpDir {
        temp_dir: TempDir,
        pub created: AtomicUsize,
    }

    impl SortTmpDir {
        /// Create the directory in `parent`.
        pub fn new(builder: &mut Builder<'_, '_>, parent: &Path) -> io::Result<Self> {
            Ok(Self {
                temp_dir: builder.tempdir_in(parent)?,
                created: AtomicUsize::new(0),
            })
        }

        /// Create the file `index`, which must not exist yet.
        pub fn create(&self, index: usize) -> io::Result<(File, TmpPath)> {
            let path = self.temp_dir.path().join(index.to_string());
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(target_os = "redox")]
            options.mode(0o600);
            let file = options.open(&path)?;
            Ok((file, TmpPath { path }))
        }

        /// Remove the files created in the directory, then the directory itself.
        /// Errors while deleting the files are ignored.
        #[cfg(not(any(target_os = "redox", target_os = "wasi")))]
        pub fn remove(&self) -> io::Result<()> {
            for index in 0..self.created.load(Ordering::Relaxed) {
                let _ = std::fs::remove_file(self.temp_dir.path().join(index.to_string()));
            }
            std::fs::remove_dir(self.temp_dir.path())
        }
    }

    /// A file created by [`super::TmpDirWrapper::next_file`].
    pub struct TmpPath {
        path: PathBuf,
    }

    impl TmpPath {
        pub fn path(&self) -> &Path {
            &self.path
        }

        pub fn open(&self) -> io::Result<File> {
            File::open(&self.path)
        }

        /// Delete the file, ignoring failures.
        pub fn remove(self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

use imp::SortTmpDir;
pub use imp::TmpPath;

#[cfg(all(test, unix))]
mod tests {
    use super::TmpDirWrapper;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// Restores the process umask on drop, so a panic in the test cannot leak the
    /// value into the rest of the binary.
    struct UmaskGuard(libc::mode_t);

    impl UmaskGuard {
        fn set(mask: libc::mode_t) -> Self {
            // SAFETY: umask(2) has no failure mode; it returns the previous value.
            Self(unsafe { libc::umask(mask) })
        }
    }

    impl Drop for UmaskGuard {
        fn drop(&mut self) {
            unsafe { libc::umask(self.0) };
        }
    }

    #[test]
    fn tmp_files_are_private_regardless_of_umask() {
        // Pin a permissive umask: under 0077 the umask alone would produce 0700 and
        // 0600, so the assertions would hold for a broken implementation too. The
        // guard restores it, and the other tests here that create files don't
        // check modes.
        let _umask = UmaskGuard::set(0o022);

        let parent = tempfile::tempdir().unwrap();
        let mut wrapper = TmpDirWrapper::new(parent.path().to_owned());
        let (_file, path) = wrapper.next_file().unwrap();

        assert_eq!(mode(path.path().parent().unwrap()), 0o700);
        assert_eq!(mode(path.path()), 0o600);
    }

    /// Once the directory's path leads to another directory, no file is created,
    /// read or removed there, even one with the name of a temporary file.
    #[test]
    #[cfg(not(target_os = "redox"))]
    fn replaced_directory_is_left_alone() {
        let parent = tempfile::tempdir().unwrap();
        let mut wrapper = TmpDirWrapper::new(parent.path().to_owned());
        let (_file, path) = wrapper.next_file().unwrap();
        let tmp_dir = path.path().parent().unwrap().to_owned();
        std::fs::rename(&tmp_dir, parent.path().join("moved")).unwrap();
        std::fs::create_dir(&tmp_dir).unwrap();
        std::fs::write(tmp_dir.join("0"), "keep").unwrap();

        assert!(wrapper.next_file().is_err());
        assert!(path.open().is_err());
        path.remove();
        drop(wrapper);

        assert_eq!(std::fs::read_to_string(tmp_dir.join("0")).unwrap(), "keep");
        assert!(!tmp_dir.join("1").exists());
    }
}
