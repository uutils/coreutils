// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::path::Path;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

/// Handle the special case where a path ends with "/."
///
/// This matches GNU/POSIX behavior where `dirname("/home/dos/.")` returns "/home/dos"
/// rather than "/home" (which would be the result of `Path::parent()` due to normalization).
/// Per POSIX.1-2017 dirname specification and GNU coreutils manual:
/// - POSIX: <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/dirname.html>
/// - GNU: <https://www.gnu.org/software/coreutils/manual/html_node/dirname-invocation.html>
///
/// dirname should do simple string manipulation without path normalization.
/// See issue #8910 and similar fix in basename (#8373, commit c5268a897).
///
/// Returns `Some(())` if the special case was handled (output already printed),
/// or `None` if normal `Path::parent()` logic should be used.
fn handle_trailing_dot(path_bytes: &[u8]) -> Option<()> {
    if !path_bytes.ends_with(b"/.") {
        return None;
    }

    // Strip the "/." suffix and print the result
    if path_bytes.len() == 2 {
        // Special case: "/." -> "/"
        print!("/");
        Some(())
    } else {
        // General case: "/home/dos/." -> "/home/dos"
        let stripped = &path_bytes[..path_bytes.len() - 2];
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let result = std::ffi::OsStr::from_bytes(stripped);
            print_verbatim(result).unwrap();
            Some(())
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, fall back to lossy conversion
            if let Ok(s) = std::str::from_utf8(stripped) {
                print!("{s}");
                Some(())
            } else {
                // Can't handle non-UTF-8 on non-Unix, fall through to normal logic
                None
            }
        }
    }
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

        if handle_trailing_dot(path_bytes).is_none() {
            // Normal path handling using Path::parent()
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
