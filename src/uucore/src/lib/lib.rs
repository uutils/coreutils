// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! library ~ (core/bundler file)
// #![deny(missing_docs)] //TODO: enable this
//
// spell-checker:ignore sigaction SIGBUS SIGSEGV

// * feature-gated external crates (re-shared as public internal modules)
#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(all(feature = "windows-sys", target_os = "windows"))]
pub extern crate windows_sys;

//## internal modules

mod features; // feature-gated code modules
mod macros; // crate macros (macro_rules-type; exported to `crate::...`)
mod mods; // core cross-platform modules
mod parser; // string parsing modules

pub use uucore_procs::*;

// * cross-platform modules
pub use crate::mods::display;
pub use crate::mods::error;
pub use crate::mods::io;
pub use crate::mods::line_ending;
pub use crate::mods::os;
pub use crate::mods::panic;
pub use crate::mods::posix;

// * string parsing modules
pub use crate::parser::parse_glob;
pub use crate::parser::parse_size;
pub use crate::parser::parse_time;
pub use crate::parser::shortcut_value_parser;

// * feature-gated modules
#[cfg(feature = "backup-control")]
pub use crate::features::backup_control;
#[cfg(feature = "buf-copy")]
pub use crate::features::buf_copy;
#[cfg(feature = "checksum")]
pub use crate::features::checksum;
#[cfg(feature = "colors")]
pub use crate::features::colors;
#[cfg(feature = "custom-tz-fmt")]
pub use crate::features::custom_tz_fmt;
#[cfg(feature = "encoding")]
pub use crate::features::encoding;
#[cfg(feature = "format")]
pub use crate::features::format;
#[cfg(feature = "fs")]
pub use crate::features::fs;
#[cfg(feature = "lines")]
pub use crate::features::lines;
#[cfg(feature = "quoting-style")]
pub use crate::features::quoting_style;
#[cfg(feature = "ranges")]
pub use crate::features::ranges;
#[cfg(feature = "ringbuffer")]
pub use crate::features::ringbuffer;
#[cfg(feature = "sum")]
pub use crate::features::sum;
#[cfg(feature = "update-control")]
pub use crate::features::update_control;
#[cfg(feature = "uptime")]
pub use crate::features::uptime;
#[cfg(feature = "version-cmp")]
pub use crate::features::version_cmp;

// * (platform-specific) feature-gated modules
// ** non-windows (i.e. Unix + Fuchsia)
#[cfg(all(not(windows), feature = "mode"))]
pub use crate::features::mode;
// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub use crate::features::entries;
#[cfg(all(unix, feature = "perms"))]
pub use crate::features::perms;
#[cfg(all(unix, any(feature = "pipes", feature = "buf-copy")))]
pub use crate::features::pipes;
#[cfg(all(unix, feature = "process"))]
pub use crate::features::process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub use crate::features::signals;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "fuchsia"),
    not(target_os = "openbsd"),
    not(target_os = "redox"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub use crate::features::utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub use crate::features::wide;

#[cfg(feature = "fsext")]
pub use crate::features::fsext;

#[cfg(all(unix, feature = "fsxattr"))]
pub use crate::features::fsxattr;

//## core functions

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::signal::{
    sigaction, SaFlags, SigAction, SigHandler::SigDfl, SigSet, Signal::SIGBUS, Signal::SIGSEGV,
};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::iter;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::str;
use std::sync::atomic::Ordering;

use once_cell::sync::Lazy;

/// Disables the custom signal handlers installed by Rust for stack-overflow handling. With those custom signal handlers processes ignore the first SIGBUS and SIGSEGV signal they receive.
/// See <https://github.com/rust-lang/rust/blob/8ac1525e091d3db28e67adcbbd6db1e1deaa37fb/src/libstd/sys/unix/stack_overflow.rs#L71-L92> for details.
#[cfg(unix)]
pub fn disable_rust_signal_handlers() -> Result<(), Errno> {
    unsafe {
        sigaction(
            SIGSEGV,
            &SigAction::new(SigDfl, SaFlags::empty(), SigSet::all()),
        )
    }?;
    unsafe {
        sigaction(
            SIGBUS,
            &SigAction::new(SigDfl, SaFlags::empty(), SigSet::all()),
        )
    }?;
    Ok(())
}

/// Execute utility code for `util`.
///
/// This macro expands to a main function that invokes the `uumain` function in `util`
/// Exits with code returned by `uumain`.
#[macro_export]
macro_rules! bin {
    ($util:ident) => {
        pub fn main() {
            use std::io::Write;
            // suppress extraneous error output for SIGPIPE failures/panics
            uucore::panic::mute_sigpipe_panic();
            // execute utility code
            let code = $util::uumain(uucore::args_os());
            // (defensively) flush stdout for utility prior to exit; see <https://github.com/rust-lang/rust/issues/23818>
            if let Err(e) = std::io::stdout().flush() {
                eprintln!("Error flushing stdout: {}", e);
            }

            std::process::exit(code);
        }
    };
}

/// Generate the usage string for clap.
///
/// This function does two things. It indents all but the first line to align
/// the lines because clap adds "Usage: " to the first line. And it replaces
/// all occurrences of `{}` with the execution phrase and returns the resulting
/// `String`. It does **not** support more advanced formatting features such
/// as `{0}`.
pub fn format_usage(s: &str) -> String {
    let s = s.replace('\n', &format!("\n{}", " ".repeat(7)));
    s.replace("{}", crate::execution_phrase())
}

/// Used to check if the utility is the second argument.
/// Used to check if we were called as a multicall binary (`coreutils <utility>`)
pub fn get_utility_is_second_arg() -> bool {
    crate::macros::UTILITY_IS_SECOND_ARG.load(Ordering::SeqCst)
}

/// Change the value of `UTILITY_IS_SECOND_ARG` to true
/// Used to specify that the utility is the second argument.
pub fn set_utility_is_second_arg() {
    crate::macros::UTILITY_IS_SECOND_ARG.store(true, Ordering::SeqCst);
}

// args_os() can be expensive to call, it copies all of argv before iterating.
// So if we want only the first arg or so it's overkill. We cache it.
static ARGV: Lazy<Vec<OsString>> = Lazy::new(|| wild::args_os().collect());

static UTIL_NAME: Lazy<String> = Lazy::new(|| {
    let base_index = usize::from(get_utility_is_second_arg());
    let is_man = usize::from(ARGV[base_index].eq("manpage"));
    let argv_index = base_index + is_man;

    ARGV[argv_index].to_string_lossy().into_owned()
});

/// Derive the utility name.
pub fn util_name() -> &'static str {
    &UTIL_NAME
}

static EXECUTION_PHRASE: Lazy<String> = Lazy::new(|| {
    if get_utility_is_second_arg() {
        ARGV.iter()
            .take(2)
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        ARGV[0].to_string_lossy().into_owned()
    }
});

/// Derive the complete execution phrase for "usage".
pub fn execution_phrase() -> &'static str {
    &EXECUTION_PHRASE
}

/// Args contains arguments passed to the utility.
/// It is a trait that extends `Iterator<Item = OsString>`.
/// It provides utility functions to collect the arguments into a `Vec<String>`.
/// The collected `Vec<String>` can be lossy or ignore invalid encoding.
pub trait Args: Iterator<Item = OsString> + Sized {
    /// Collects the iterator into a `Vec<String>`, lossily converting the `OsString`s to `Strings`.
    fn collect_lossy(self) -> Vec<String> {
        self.map(|s| s.to_string_lossy().into_owned()).collect()
    }

    /// Collects the iterator into a `Vec<String>`, removing any elements that contain invalid encoding.
    fn collect_ignore(self) -> Vec<String> {
        self.filter_map(|s| s.into_string().ok()).collect()
    }
}

impl<T: Iterator<Item = OsString> + Sized> Args for T {}

/// Returns an iterator over the command line arguments as `OsString`s.
/// args_os() can be expensive to call
pub fn args_os() -> impl Iterator<Item = OsString> {
    ARGV.iter().cloned()
}

/// Read a line from stdin and check whether the first character is `'y'` or `'Y'`
pub fn read_yes() -> bool {
    let mut s = String::new();
    match std::io::stdin().read_line(&mut s) {
        Ok(_) => matches!(s.chars().next(), Some('y' | 'Y')),
        _ => false,
    }
}

/// Converts an `OsStr` to a UTF-8 `&[u8]`.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the string can't be coerced to UTF-8.
pub fn os_str_as_bytes(os_string: &OsStr) -> mods::error::UResult<&[u8]> {
    #[cfg(unix)]
    let bytes = os_string.as_bytes();

    #[cfg(not(unix))]
    let bytes = os_string
        .to_str()
        .ok_or_else(|| {
            mods::error::UUsageError::new(1, "invalid UTF-8 was detected in one or more arguments")
        })?
        .as_bytes();

    Ok(bytes)
}

/// Performs a potentially lossy conversion from `OsStr` to UTF-8 bytes.
///
/// This is always lossless on unix platforms,
/// and wraps [`OsStr::to_string_lossy`] on non-unix platforms.
pub fn os_str_as_bytes_lossy(os_string: &OsStr) -> Cow<[u8]> {
    #[cfg(unix)]
    let bytes = Cow::from(os_string.as_bytes());

    #[cfg(not(unix))]
    let bytes = match os_string.to_string_lossy() {
        Cow::Borrowed(slice) => Cow::from(slice.as_bytes()),
        Cow::Owned(owned) => Cow::from(owned.into_bytes()),
    };

    bytes
}

/// Converts a `&[u8]` to an `&OsStr`,
/// or parses it as UTF-8 into an [`OsString`] on non-unix platforms.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the bytes can't be parsed as UTF-8.
pub fn os_str_from_bytes(bytes: &[u8]) -> mods::error::UResult<Cow<'_, OsStr>> {
    #[cfg(unix)]
    let os_str = Cow::Borrowed(OsStr::from_bytes(bytes));
    #[cfg(not(unix))]
    let os_str = Cow::Owned(OsString::from(str::from_utf8(bytes).map_err(|_| {
        mods::error::UUsageError::new(1, "Unable to transform bytes into OsStr")
    })?));

    Ok(os_str)
}

/// Converts a `Vec<u8>` into an `OsString`, parsing as UTF-8 on non-unix platforms.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the bytes can't be parsed as UTF-8.
pub fn os_string_from_vec(vec: Vec<u8>) -> mods::error::UResult<OsString> {
    #[cfg(unix)]
    let s = OsString::from_vec(vec);
    #[cfg(not(unix))]
    let s = OsString::from(String::from_utf8(vec).map_err(|_| {
        mods::error::UUsageError::new(1, "invalid UTF-8 was detected in one or more arguments")
    })?);

    Ok(s)
}

/// Equivalent to `std::BufRead::lines` which outputs each line as a `Vec<u8>`,
/// which avoids panicking on non UTF-8 input.
pub fn read_byte_lines<R: std::io::Read>(
    mut buf_reader: BufReader<R>,
) -> impl Iterator<Item = Vec<u8>> {
    iter::from_fn(move || {
        let mut buf = Vec::with_capacity(256);
        let size = buf_reader.read_until(b'\n', &mut buf).ok()?;

        if size == 0 {
            return None;
        }

        // Trim (\r)\n
        if buf.ends_with(b"\n") {
            buf.pop();
            if buf.ends_with(b"\r") {
                buf.pop();
            }
        }

        Some(buf)
    })
}

/// Equivalent to `std::BufRead::lines` which outputs each line as an `OsString`
/// This won't panic on non UTF-8 characters on Unix,
/// but it still will on Windows.
pub fn read_os_string_lines<R: std::io::Read>(
    buf_reader: BufReader<R>,
) -> impl Iterator<Item = OsString> {
    read_byte_lines(buf_reader).map(|byte_line| os_string_from_vec(byte_line).expect("UTF-8 error"))
}

/// Prompt the user with a formatted string and returns `true` if they reply `'y'` or `'Y'`
///
/// This macro functions accepts the same syntax as `format!`. The prompt is written to
/// `stderr`. A space is also printed at the end for nice spacing between the prompt and
/// the user input. Any input starting with `'y'` or `'Y'` is interpreted as `yes`.
///
/// # Examples
/// ```
/// use uucore::prompt_yes;
/// let file = "foo.rs";
/// prompt_yes!("Do you want to delete '{}'?", file);
/// ```
/// will print something like below to `stderr` (with `util_name` substituted by the actual
/// util name) and will wait for user input.
/// ```txt
/// util_name: Do you want to delete 'foo.rs'?
/// ```
#[macro_export]
macro_rules! prompt_yes(
    ($($args:tt)+) => ({
        use std::io::Write;
        eprint!("{}: ", uucore::util_name());
        eprint!($($args)+);
        eprint!(" ");
        let res = std::io::stderr().flush().map_err(|err| {
            $crate::error::USimpleError::new(1, err.to_string())
        });
        uucore::show_if_err!(res);
        uucore::read_yes()
    })
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn make_os_vec(os_str: &OsStr) -> Vec<OsString> {
        vec![
            OsString::from("test"),
            OsString::from("สวัสดี"), // spell-checker:disable-line
            os_str.to_os_string(),
        ]
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn test_invalid_utf8_args_lossy(os_str: &OsStr) {
        // assert our string is invalid utf8
        assert!(os_str.to_os_string().into_string().is_err());
        let test_vec = make_os_vec(os_str);
        let collected_to_str = test_vec.clone().into_iter().collect_lossy();
        // conservation of length - when accepting lossy conversion no arguments may be dropped
        assert_eq!(collected_to_str.len(), test_vec.len());
        // first indices identical
        for index in 0..2 {
            assert_eq!(collected_to_str[index], test_vec[index].to_str().unwrap());
        }
        // lossy conversion for string with illegal encoding is done
        assert_eq!(
            *collected_to_str[2],
            os_str.to_os_string().to_string_lossy()
        );
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn test_invalid_utf8_args_ignore(os_str: &OsStr) {
        // assert our string is invalid utf8
        assert!(os_str.to_os_string().into_string().is_err());
        let test_vec = make_os_vec(os_str);
        let collected_to_str = test_vec.clone().into_iter().collect_ignore();
        // assert that the broken entry is filtered out
        assert_eq!(collected_to_str.len(), test_vec.len() - 1);
        // assert that the unbroken indices are converted as expected
        for index in 0..2 {
            assert_eq!(
                collected_to_str.get(index).unwrap(),
                test_vec.get(index).unwrap().to_str().unwrap()
            );
        }
    }

    #[test]
    fn valid_utf8_encoding_args() {
        // create a vector containing only correct encoding
        let test_vec = make_os_vec(&OsString::from("test2"));
        // expect complete conversion without losses, even when lossy conversion is accepted
        let _ = test_vec.into_iter().collect_lossy();
    }

    #[cfg(any(unix, target_os = "redox"))]
    #[test]
    fn invalid_utf8_args_unix() {
        use std::os::unix::ffi::OsStrExt;

        let source = [0x66, 0x6f, 0x80, 0x6f];
        let os_str = OsStr::from_bytes(&source[..]);
        test_invalid_utf8_args_lossy(os_str);
        test_invalid_utf8_args_ignore(os_str);
    }

    #[test]
    fn test_format_usage() {
        assert_eq!(format_usage("expr EXPRESSION"), "expr EXPRESSION");
        assert_eq!(
            format_usage("expr EXPRESSION\nexpr OPTION"),
            "expr EXPRESSION\n       expr OPTION"
        );
    }
}
