// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::{Write as _, stdout};
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

#[cfg(not(windows))]
fn is_sep(b: u8) -> bool {
    std::path::is_separator(b as char)
}

#[cfg(windows)]
fn is_sep(b: u8) -> bool {
    // `std::path::is_separator` does not optimize well on Windows. It is essentially
    // `c.is_ascii() && is_sep_byte(c as u8)`, and LLVM does not make the assumption
    // that the `c.is_ascii()` is unintentional here. The result is a useless branch.
    b == b'/' || b == b'\\'
}

#[cfg(not(windows))]
fn root_prefix_len(_: &[u8]) -> usize {
    0
}

#[cfg(windows)]
fn root_prefix_len(bytes: &[u8]) -> usize {
    use std::ffi::OsStr;
    use std::path::Component;
    use std::path::Path;

    // SAFETY: These bytes came from `OsStr` via `uucore::os_str_as_bytes`.
    let path = Path::new(unsafe { OsStr::from_encoded_bytes_unchecked(bytes) });
    let Some(Component::Prefix(p)) = path.components().next() else {
        return 0;
    };

    let mut len = p.as_os_str().len();

    // Include the root directory separator after the prefix, if present.
    if len > 0 && len < bytes.len() && is_sep(bytes[len]) {
        len += 1;
    }

    len
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
/// - `C:\foo\bar` → `C:\foo` (Windows)
/// - `C:\foo` → `C:\` (Windows)
/// - `\\server\share\foo` → `\\server\share\` (Windows)
///
/// Per POSIX.1-2017 dirname specification and GNU coreutils manual:
/// - POSIX: <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/dirname.html>
/// - GNU: <https://www.gnu.org/software/coreutils/manual/html_node/dirname-invocation.html>
///
/// See issue #8910 and similar fix in basename (#8373, commit c5268a897).
fn dirname_string_manipulation(path_bytes: &[u8]) -> &[u8] {
    if path_bytes.is_empty() {
        return b".";
    }

    let mut bytes = path_bytes;
    let root_len = root_prefix_len(bytes);

    // Step 1: Strip trailing slashes (but not if the entire path is slashes)
    if bytes[root_len..].iter().copied().all(is_sep) {
        return &bytes[..root_len.max(1)];
    }

    while bytes.len() > root_len.max(1) && is_sep(bytes[bytes.len() - 1]) {
        bytes = &bytes[..bytes.len() - 1];
    }

    // Step 2: Check if it ends with `/.` and strip the `/+.` pattern
    if bytes.ends_with(b".") && bytes.len() >= 2 && is_sep(bytes[bytes.len() - 2]) {
        let dot_pos = bytes.len() - 1;
        // Find where the slashes before the dot start
        let mut slash_start = dot_pos - 1;
        while slash_start > 0 && is_sep(bytes[slash_start - 1]) {
            slash_start -= 1;
        }
        // Return the stripped result
        if slash_start == 0 {
            // Result would be empty
            return if is_sep(path_bytes[0]) {
                &path_bytes[..1]
            } else {
                b"."
            };
        }
        if slash_start < root_len {
            // NOTE: This gets optimized away on Unix, because root_len is 0.
            return &path_bytes[..root_len];
        }
        return &bytes[..slash_start];
    }

    // Step 3: Normal dirname - find last / and remove everything after it
    if let Some(last_slash_pos) = bytes.iter().copied().rposition(is_sep) {
        // Found a slash, remove everything after it
        let mut result = &bytes[..last_slash_pos];

        // Strip trailing slashes from result (but keep at least one if at the start)
        while result.len() > 1 && is_sep(result[result.len() - 1]) {
            result = &result[..result.len() - 1];
        }

        if result.is_empty() {
            return &bytes[..1];
        }

        if result.len() < root_len {
            // NOTE: This gets optimized away on Unix, because root_len is 0.
            return &bytes[..root_len];
        }
        return result;
    }

    // No slash found, return "."
    if root_len > 0 {
        // NOTE: This gets optimized away on Unix, because root_len is 0.
        return &bytes[..root_len];
    }
    b"."
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    // todo: use .required(true) of clap and sed GnuTests to provide better error message
    let dirnames: Vec<OsString> = matches
        .get_many::<OsString>(options::DIR)
        .ok_or_else(|| UUsageError::new(1, translate!("dirname-missing-operand")))?
        .cloned()
        .collect();

    for path in &dirnames {
        let path_bytes = uucore::os_str_as_bytes(path.as_os_str()).unwrap_or(&[]);
        let result = dirname_string_manipulation(path_bytes);

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let result_os = std::ffi::OsStr::from_bytes(result);
            print_verbatim(result_os)?;
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, fall back to lossy conversion
            if let Ok(s) = std::str::from_utf8(result) {
                write!(stdout(), "{s}")?;
            } else {
                // Fallback for non-UTF-8 paths on non-Unix systems
                write!(stdout(), ".")?;
            }
        }

        write!(stdout(), "{line_ending}")?;
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("dirname")
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
