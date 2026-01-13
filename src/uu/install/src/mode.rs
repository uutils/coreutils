// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs;
#[cfg(all(unix, not(target_os = "redox")))]
use std::os::fd::{AsFd, AsRawFd};
use std::path::Path;
#[cfg(all(unix, not(target_os = "redox")))]
use uucore::libc;
use uucore::translate;

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
    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;
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

    let mut components = path.components().peekable();
    let mut current_fd: Option<OwnedFd> = None;
    let mut last_component: Option<CString> = None;

    if path.is_absolute() {
        let fd = open(Path::new("/"), dir_open_flags(), Mode::empty()).map_err(errno_to_io)?;
        current_fd = Some(fd);
        while matches!(components.peek(), Some(Component::RootDir)) {
            components.next();
        }
    }

    while let Some(component) = components.next() {
        match component {
            Component::CurDir => {
                if components.peek().is_none() {
                    last_component = Some(CString::new(".").unwrap());
                }
            }
            Component::RootDir => {}
            Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsupported path prefix",
                ));
            }
            Component::ParentDir => {
                if components.peek().is_none() {
                    last_component = Some(CString::new("..").unwrap());
                } else {
                    let base_fd = current_fd.as_ref().map_or_else(
                        || unsafe { BorrowedFd::borrow_raw(libc::AT_FDCWD) },
                        |fd| fd.as_fd(),
                    );

                    let fd = openat(base_fd, OsStr::new(".."), dir_open_flags(), Mode::empty())
                        .map_err(errno_to_io)?;
                    current_fd = Some(fd);
                }
            }
            Component::Normal(name) => {
                let base_fd = current_fd.as_ref().map_or_else(
                    || unsafe { BorrowedFd::borrow_raw(libc::AT_FDCWD) },
                    |fd| fd.as_fd(),
                );

                let is_last = components.peek().is_none();

                let name_cstr = CString::new(name.as_bytes()).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "path segment contains null byte",
                    )
                })?;

                if is_last {
                    last_component = Some(name_cstr);
                } else {
                    let fd = openat(
                        base_fd,
                        name_cstr.as_c_str(),
                        dir_open_flags(),
                        Mode::empty(),
                    )
                    .map_err(errno_to_io)?;
                    current_fd = Some(fd);
                }
            }
        }
    }

    let name_cstring = match last_component {
        Some(name) => name,
        None => CString::new(".").unwrap(),
    };

    let dirfd_raw = current_fd
        .as_ref()
        .map_or(libc::AT_FDCWD, |fd| fd.as_fd().as_raw_fd());

    let result =
        unsafe { libc::fchmodat(dirfd_raw, name_cstring.as_ptr(), mode as libc::mode_t, 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno_to_io(Errno::last()))
    }
}
