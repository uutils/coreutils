// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::borrow::Cow;
use std::ffi::OsString;
#[cfg(unix)]
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

/// Perform dirname as pure string manipulation per POSIX/GNU behavior.
///
/// dirname should NOT normalize paths. It does simple string manipulation:
/// 1. Strip trailing slashes (unless path is all slashes)
/// 2. If ends with `/.` (possibly `//.` or `///.`), strip the `/+.` pattern
/// 3. Otherwise, remove everything after the last `/`
/// 4. If no `/` found, return `.`
/// 5. Strip trailing slashes from result (unless result would be empty)
///
/// Examples:
/// - `foo/.` → `foo`
/// - `foo/./bar` → `foo/.`
/// - `foo/bar` → `foo`
/// - `a/b/c` → `a/b`
///
/// Per POSIX.1-2017 dirname specification and GNU coreutils manual:
/// - POSIX: <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/dirname.html>
/// - GNU: <https://www.gnu.org/software/coreutils/manual/html_node/dirname-invocation.html>
///
/// See issue #8910 and similar fix in basename (#8373, commit c5268a897).
fn dirname_string_manipulation(path_bytes: &[u8]) -> Cow<'_, [u8]> {
    if path_bytes.is_empty() {
        return Cow::Borrowed(b".");
    }

    let mut bytes = path_bytes;

    // Step 1: Strip trailing slashes (but not if the entire path is slashes)
    let all_slashes = bytes.iter().all(|&b| b == b'/');
    if all_slashes {
        return Cow::Borrowed(b"/");
    }

    while bytes.len() > 1 && bytes.ends_with(b"/") {
        bytes = &bytes[..bytes.len() - 1];
    }

    // Step 2: Check if it ends with `/.` and strip the `/+.` pattern
    if bytes.ends_with(b".") && bytes.len() >= 2 {
        let dot_pos = bytes.len() - 1;
        if bytes[dot_pos - 1] == b'/' {
            // Find where the slashes before the dot start
            let mut slash_start = dot_pos - 1;
            while slash_start > 0 && bytes[slash_start - 1] == b'/' {
                slash_start -= 1;
            }
            // Return the stripped result
            if slash_start == 0 {
                // Result would be empty
                return if path_bytes.starts_with(b"/") {
                    Cow::Borrowed(b"/")
                } else {
                    Cow::Borrowed(b".")
                };
            }
            return Cow::Borrowed(&bytes[..slash_start]);
        }
    }

    // Step 3: Normal dirname - find last / and remove everything after it
    if let Some(last_slash_pos) = bytes.iter().rposition(|&b| b == b'/') {
        // Found a slash, remove everything after it
        let mut result = &bytes[..last_slash_pos];

        // Strip trailing slashes from result (but keep at least one if at the start)
        while result.len() > 1 && result.ends_with(b"/") {
            result = &result[..result.len() - 1];
        }

        if result.is_empty() {
            return Cow::Borrowed(b"/");
        }

        return Cow::Borrowed(result);
    }

    // No slash found, return "."
    Cow::Borrowed(b".")
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
        let result = dirname_string_manipulation(path_bytes);

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt as _;
            let result_os = std::ffi::OsStr::from_bytes(&result);
            print_verbatim(result_os).unwrap();
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, fall back to lossy conversion
            if let Ok(s) = std::str::from_utf8(&result) {
                print!("{s}");
            } else {
                // Fallback for non-UTF-8 paths on non-Unix systems
                print!(".");
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
