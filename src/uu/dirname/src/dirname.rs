// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

/// This matches GNU/POSIX behavior where `dirname("/home/dos/.")` returns "/home/dos"
/// rather than "/home" (which would be the result of `Path::parent()` due to normalization).
/// Per POSIX.1-2017 dirname specification and GNU coreutils manual:
/// - POSIX: <https://pubs.opengroup.org/onlinepubs/9699919799/utilities/dirname.html>
/// - GNU: <https://www.gnu.org/software/coreutils/manual/html_node/dirname-invocation.html>
///
/// dirname should do simple string manipulation without path normalization.
/// See issue #8910 and similar fix in basename (#8373, commit c5268a897).
fn dirname_bytes(path: &[u8]) -> &[u8] {
    // Skip any trailing slashes
    let Some(i) = path.iter().rposition(|&b| b != b'/') else {
        return if path.is_empty() { b"." } else { b"/" }; // path was all slashes
    };
    // Skip final component
    let Some(i) = path[..i].iter().rposition(|&b| b == b'/') else {
        return b"."; // path had one relative component
    };
    // Skip any remaining trailing slashes
    let Some(i) = path[..i].iter().rposition(|&b| b != b'/') else {
        return b"/"; // path had one absolute component
    };
    &path[..=i]
}

fn dirname(path: &OsStr) -> &OsStr {
    let path_bytes = path.as_encoded_bytes();
    let dir_bytes = dirname_bytes(path_bytes);
    // SAFETY: The internal encoding of OsStr is documented to be a
    // self-synchronizing superset of UTF-8. Since dir_bytes was computed as a
    // subslice of path_bytes adjacent only to b'/', it is also valid as an
    // OsStr. (The experimental os_str_slice feature may allow this to be
    // rewritten without unsafe in the future.)
    unsafe { OsStr::from_encoded_bytes_unchecked(dir_bytes) }
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
        print_verbatim(dirname(path.as_os_str()))?;
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
