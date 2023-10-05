// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use uucore::mode;

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
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|err| {
        show_error!("{}: chmod failed with error {}", path.maybe_quote(), err);
    })
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
