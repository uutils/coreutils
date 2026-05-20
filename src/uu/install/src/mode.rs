// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// chmod a file or directory on UNIX.
///
/// Adapted from mkdir.rs.  Handles own error printing.
///
/// Uses libc::chmod directly instead of fs::set_permissions to properly
/// handle special mode bits (setuid, setgid, sticky) when running as root.
///
#[cfg(any(unix, target_os = "redox"))]
pub fn chmod(path: &Path, mode: u32) -> Result<(), std::io::Error> {
    use std::ffi::CString;
    use uucore::libc;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // Use libc::chmod directly to properly handle all mode bits including special bits
    if unsafe { libc::chmod(c_path.as_ptr(), mode as libc::mode_t) } != 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// chmod a file or directory on Windows.
///
/// Adapted from mkdir.rs.
///
#[cfg(windows)]
pub fn chmod(path: &Path, mode: u32) -> Result<(), std::io::Error> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}
