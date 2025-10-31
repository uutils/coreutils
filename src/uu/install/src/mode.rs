// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#[cfg(any(unix, target_os = "redox"))]
use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use uucore::mode;
#[cfg(any(unix, target_os = "redox"))]
use uucore::translate;

/// Takes a user-supplied string and tries to parse to u16 mode bitmask.
#[cfg(not(windows))]
pub fn parse(mode_string: &str, considering_dir: bool, umask: u32) -> Result<u32, String> {
    if mode_string.chars().any(|c| c.is_ascii_digit()) {
        mode::parse_numeric(0, mode_string, considering_dir)
    } else {
        mode::parse_symbolic(0, mode_string, umask, considering_dir)
    }
}

#[cfg(windows)]
pub fn parse(mode_string: &str, _considering_dir: bool, _umask: u32) -> Result<u32, String> {
    if mode_string.chars().all(|c| c.is_ascii_digit()) {
        u32::from_str_radix(mode_string, 8)
            .map_err(|_| format!("invalid numeric mode '{mode_string}'"))
    } else {
        Err(format!(
            "symbolic modes like '{mode_string}' are not supported on Windows"
        ))
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
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|err| {
        show_error!(
            "{}",
            translate!("install-error-chmod-failed-detailed", "path" => path.maybe_quote(), "error" => err)
        );
    })
}

/// chmod a file or directory on Windows.
///
/// Adapted from mkdir.rs.
///
#[cfg(windows)]
pub fn chmod(_path: &Path, _mode: u32) -> Result<(), ()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}
