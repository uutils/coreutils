// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use uucore::mode;
use uucore::translate;

/// Takes a user-supplied string and tries to parse to u16 mode bitmask.
pub fn parse(mode_string: &str, considering_dir: bool, umask: u32) -> Result<u32, String> {
    if mode_string.chars().any(|c| c.is_ascii_digit()) {
        mode::parse_numeric(0, mode_string, considering_dir)
    } else {
        mode::parse_symbolic(0, mode_string, umask, considering_dir)
    }
}

/// chmod a file or directory on UNIX.
///
/// Adapted from mkdir.rs.  Handles own error printing.
///
#[cfg(any(unix, target_os = "redox"))]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    use std::os::unix::fs::PermissionsExt;
    use uucore::{display::Quotable, show_error};
    match fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(all(unix, not(target_os = "redox")))]
            {
                if err.raw_os_error() == Some(libc::ENAMETOOLONG) {
                    match chmod_long_path(path, mode) {
                        Ok(()) => return Ok(()),
                        Err(fallback_err) => {
                            show_error!(
                                "{}",
                                translate!(
                                    "install-error-chmod-failed-detailed",
                                    "path" => path.maybe_quote(),
                                    "error" => fallback_err
                                )
                            );
                            return Err(());
                        }
                    }
                }
            }

            show_error!(
                "{}",
                translate!(
                    "install-error-chmod-failed-detailed",
                    "path" => path.maybe_quote(),
                    "error" => err
                )
            );
            Err(())
        }
    }
}

/// chmod a file or directory on Windows.
///
/// Adapted from mkdir.rs.
///
#[cfg(windows)]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

#[cfg(all(unix, not(target_os = "redox")))]
fn chmod_long_path(path: &Path, mode: u32) -> std::io::Result<()> {
    use nix::errno::Errno;
    use nix::fcntl::{open, openat, OFlag};
    use nix::sys::stat::{Mode, fchmod};
    use std::ffi::{CString, OsStr};
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{BorrowedFd, OwnedFd};
    use std::path::{Component, Path};

    fn errno_to_io(err: Errno) -> io::Error {
        io::Error::from_raw_os_error(err as i32)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn dir_open_flags() -> OFlag {
        OFlag::O_PATH | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    fn dir_open_flags() -> OFlag {
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn node_open_flags() -> OFlag {
        OFlag::O_PATH | OFlag::O_CLOEXEC
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    fn node_open_flags() -> OFlag {
        OFlag::O_RDONLY | OFlag::O_CLOEXEC
    }

    let mut components = path.components().peekable();
    let mut current_fd: Option<OwnedFd> = None;

    if path.is_absolute() {
        let fd = open(Path::new("/"), dir_open_flags(), Mode::empty()).map_err(errno_to_io)?;
        current_fd = Some(fd);
        while matches!(components.peek(), Some(Component::RootDir)) {
            components.next();
        }
    }

    while let Some(component) = components.next() {
        match component {
            Component::CurDir => {}
            Component::RootDir => {}
            Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsupported path prefix",
                ));
            }
            Component::ParentDir => {
                let base_fd = current_fd
                    .as_ref()
                    .map(|fd| fd.as_fd())
                    .unwrap_or_else(|| unsafe {
                        BorrowedFd::borrow_raw(libc::AT_FDCWD)
                    });

                let fd =
                    openat(base_fd, OsStr::new(".."), dir_open_flags(), Mode::empty())
                        .map_err(errno_to_io)?;
                current_fd = Some(fd);
            }
            Component::Normal(name) => {
                let base_fd = current_fd
                    .as_ref()
                    .map(|fd| fd.as_fd())
                    .unwrap_or_else(|| unsafe {
                        BorrowedFd::borrow_raw(libc::AT_FDCWD)
                    });

                let is_last = components.peek().is_none();

                let flags = if is_last {
                    node_open_flags()
                } else {
                    dir_open_flags()
                };

                let name_cstr = CString::new(name.as_bytes()).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "path segment contains null byte")
                })?;

                let fd = openat(base_fd, name_cstr.as_c_str(), flags, Mode::empty())
                    .map_err(errno_to_io)?;
                current_fd = Some(fd);
            }
        }
    }

    let fd = current_fd.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path does not reference an entry")
    })?;

    fchmod(&fd, Mode::from_bits_truncate(mode)).map_err(errno_to_io)
}
