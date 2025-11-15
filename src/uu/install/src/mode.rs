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
/// Supports comma-separated mode strings like "ug+rwX,o+rX" (same as chmod).
pub fn parse(mode_string: &str, considering_dir: bool, umask: u32) -> Result<u32, String> {
    // Split by commas and process each mode part sequentially
    let mut current_mode: u32 = 0;

    for mode_part in mode_string.split(',') {
        let mode_part = mode_part.trim();
        if mode_part.is_empty() {
            continue;
        }

        current_mode = if mode_part.chars().any(|c| c.is_ascii_digit()) {
            mode::parse_numeric(current_mode, mode_part, considering_dir)?
        } else {
            mode::parse_symbolic(current_mode, mode_part, umask, considering_dir)?
        };
    }

    Ok(current_mode)
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
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}
