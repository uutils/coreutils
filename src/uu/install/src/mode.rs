// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fs;
use std::path::Path;
use uucore::translate;

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
