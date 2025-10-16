// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

#[cfg(not(unix))]
use std::path::Path;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

/// Compute dirname following POSIX/GNU behavior
///
/// This implements the POSIX dirname algorithm without path normalization.
/// Per POSIX.1-2017 dirname specification and GNU coreutils manual:
/// - POSIX: <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/dirname.html>
/// - GNU: <https://www.gnu.org/software/coreutils/manual/html_node/dirname-invocation.html>
///
/// The algorithm:
/// 1. Remove trailing '/' characters
/// 2. If the path ends with "/.", remove it (handles foo/., foo//., foo///., etc.)
/// 3. Remove any remaining trailing '/' characters
/// 4. Apply standard dirname logic (find last '/', return everything before it)
///
/// See issues #8910 and #8924, and similar fix in basename (#8373, commit c5268a897).
fn compute_dirname(path_bytes: &[u8]) -> Vec<u8> {
    // Handle empty path
    if path_bytes.is_empty() {
        return b".".to_vec();
    }

    // Special case: "//" stays as "/" per POSIX
    if path_bytes == b"//" {
        return b"/".to_vec();
    }

    let mut path = path_bytes.to_vec();

    // If path consists entirely of slashes, return single slash
    if path.iter().all(|&b| b == b'/') {
        return b"/".to_vec();
    }

    // Step 1: Remove trailing slashes (but keep at least one character)
    while path.len() > 1 && path.last() == Some(&b'/') {
        path.pop();
    }

    // Step 2: Check if path ends with "/." and handle specially
    // This handles foo/., foo//., foo///., and foo/./ (after step 1) etc.
    if path.len() >= 2 && path[path.len() - 1] == b'.' && path[path.len() - 2] == b'/' {
        // Remember if the original path was absolute (for handling "/." -> "/")
        let was_absolute = path[0] == b'/';

        // Remove the "/." suffix
        path.truncate(path.len() - 2);

        // Remove any additional trailing slashes that might remain (e.g., foo//. -> foo/)
        while path.len() > 1 && path.last() == Some(&b'/') {
            path.pop();
        }

        // Handle edge cases: if we're left with nothing or just slashes
        if path.is_empty() {
            // If it was an absolute path like "/.", return "/"
            // Otherwise, return "."
            return if was_absolute {
                b"/".to_vec()
            } else {
                b".".to_vec()
            };
        }
        if path.iter().all(|&b| b == b'/') {
            return b"/".to_vec();
        }

        // What remains IS the dirname for paths ending with "/.".
        // Example: "foo/bar/." -> "foo/bar", "foo//." -> "foo"
        return path;
    }

    // Step 3: Standard dirname logic - find last '/' and return everything before it
    if let Some(pos) = path.iter().rposition(|&b| b == b'/') {
        if pos == 0 {
            // The slash is at the beginning, dirname is "/"
            return b"/".to_vec();
        }
        path.truncate(pos);
        return path;
    }

    // No slash found, dirname is "."
    b".".to_vec()
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let dirnames: Vec<OsString> = matches
        .get_many::<OsString>(options::DIR)
        .unwrap_or_default()
        .cloned()
        .collect();

    if dirnames.is_empty() {
        return Err(UUsageError::new(1, translate!("dirname-missing-operand")));
    }

    for path in &dirnames {
        let path_bytes = uucore::os_str_as_bytes(path.as_os_str()).unwrap_or(&[]);

        // Compute dirname using POSIX-compliant algorithm
        let dirname_bytes = compute_dirname(path_bytes);

        // Print the result
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let result = std::ffi::OsStr::from_bytes(&dirname_bytes);
            print_verbatim(result).unwrap();
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, fall back to lossy conversion
            if let Ok(s) = std::str::from_utf8(&dirname_bytes) {
                print!("{s}");
            } else {
                // Fallback for non-UTF-8 on non-Unix: use Path::parent() as before
                let p = Path::new(path);
                match p.parent() {
                    Some(d) => {
                        if d.components().next().is_none() {
                            print!(".");
                        } else {
                            print_verbatim(d).unwrap();
                        }
                    }
                    None => {
                        if p.is_absolute() || path.as_os_str() == "/" {
                            print!("/");
                        } else {
                            print!(".");
                        }
                    }
                }
            }
        }

        print!("{line_ending}");
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(translate!("dirname-about"))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("dirname-usage")))
        .args_override_self(true)
        .infer_long_args(true)
        .after_help(translate!("dirname-after-help"))
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("dirname-zero-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIR)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString)),
        )
}
