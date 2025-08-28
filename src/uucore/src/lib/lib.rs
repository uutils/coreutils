// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! library ~ (core/bundler file)
// #![deny(missing_docs)] //TODO: enable this
//
// spell-checker:ignore sigaction SIGBUS SIGSEGV extendedbigdecimal myutil logind

// * feature-gated external crates (re-shared as public internal modules)
#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(all(feature = "windows-sys", target_os = "windows"))]
pub extern crate windows_sys;

//## internal modules

mod features; // feature-gated code modules
mod macros; // crate macros (macro_rules-type; exported to `crate::...`)
mod mods; // core cross-platform modules

pub use uucore_procs::*;

// * cross-platform modules
pub use crate::mods::clap_localization;
pub use crate::mods::clap_localization::LocalizedCommand;
pub use crate::mods::display;
pub use crate::mods::error;
#[cfg(feature = "fs")]
pub use crate::mods::io;
pub use crate::mods::line_ending;
pub use crate::mods::locale;
pub use crate::mods::os;
pub use crate::mods::panic;
pub use crate::mods::posix;

// * feature-gated modules
#[cfg(feature = "backup-control")]
pub use crate::features::backup_control;
#[cfg(feature = "buf-copy")]
pub use crate::features::buf_copy;
#[cfg(feature = "checksum")]
pub use crate::features::checksum;
#[cfg(feature = "colors")]
pub use crate::features::colors;
#[cfg(feature = "encoding")]
pub use crate::features::encoding;
#[cfg(feature = "extendedbigdecimal")]
pub use crate::features::extendedbigdecimal;
#[cfg(feature = "fast-inc")]
pub use crate::features::fast_inc;
#[cfg(feature = "format")]
pub use crate::features::format;
#[cfg(feature = "fs")]
pub use crate::features::fs;
#[cfg(feature = "i18n-common")]
pub use crate::features::i18n;
#[cfg(feature = "lines")]
pub use crate::features::lines;
#[cfg(feature = "parser")]
pub use crate::features::parser;
#[cfg(feature = "quoting-style")]
pub use crate::features::quoting_style;
#[cfg(feature = "ranges")]
pub use crate::features::ranges;
#[cfg(feature = "ringbuffer")]
pub use crate::features::ringbuffer;
#[cfg(feature = "sum")]
pub use crate::features::sum;
#[cfg(feature = "feat_systemd_logind")]
pub use crate::features::systemd_logind;
#[cfg(feature = "time")]
pub use crate::features::time;
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

#[cfg(all(target_os = "linux", feature = "selinux"))]
pub use crate::features::selinux;

//## core functions

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::signal::{
    SaFlags, SigAction, SigHandler::SigDfl, SigSet, Signal::SIGBUS, Signal::SIGSEGV, sigaction,
};
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader};
use std::iter;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::str;
use std::str::Utf8Chunk;
use std::sync::{LazyLock, atomic::Ordering};

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

pub fn get_canonical_util_name(util_name: &str) -> &str {
    // remove the "uu_" prefix
    let util_name = &util_name[3..];
    match util_name {
        // uu_test aliases - '[' is an alias for test
        "[" => "test",

        // hashsum aliases - all these hash commands are aliases for hashsum
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum"
        | "sha3sum" | "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum"
        | "shake128sum" | "shake256sum" | "b2sum" | "b3sum" => "hashsum",

        "dir" => "ls", // dir is an alias for ls

        // Default case - return the util name as is
        _ => util_name,
    }
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
            use uucore::locale;
            // suppress extraneous error output for SIGPIPE failures/panics
            uucore::panic::mute_sigpipe_panic();
            locale::setup_localization(uucore::get_canonical_util_name(stringify!($util)))
                .unwrap_or_else(|err| {
                    match err {
                        uucore::locale::LocalizationError::ParseResource {
                            error: err_msg,
                            snippet,
                        } => eprintln!("Localization parse error at {snippet}: {err_msg:?}"),
                        other => eprintln!("Could not init the localization system: {other}"),
                    }
                    std::process::exit(99)
                });

            // execute utility code
            let code = $util::uumain(uucore::args_os());
            // (defensively) flush stdout for utility prior to exit; see <https://github.com/rust-lang/rust/issues/23818>
            if let Err(e) = std::io::stdout().flush() {
                eprintln!("Error flushing stdout: {e}");
            }

            std::process::exit(code);
        }
    };
}

/// Generate the version string for clap.
///
/// The generated string has the format `(<project name>) <version>`, for
/// example: "(uutils coreutils) 0.30.0". clap will then prefix it with the util name.
#[macro_export]
macro_rules! crate_version {
    () => {
        concat!("(uutils coreutils) ", env!("CARGO_PKG_VERSION"))
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

/// Creates a localized help template for clap commands.
///
/// This function returns a help template that uses the localized
/// "Usage:" label from the translation files. This ensures consistent
/// localization across all utilities.
///
/// Note: We avoid using clap's `{usage-heading}` placeholder because it is
/// hardcoded to "Usage:" and cannot be localized. Instead, we manually
/// construct the usage line with the localized label.
///
/// # Parameters
/// - `util_name`: The name of the utility (for localization setup)
///
/// # Example
/// ```no_run
/// use clap::Command;
/// use uucore::localized_help_template;
///
/// let app = Command::new("myutil")
///     .help_template(localized_help_template("myutil"));
/// ```
pub fn localized_help_template(util_name: &str) -> clap::builder::StyledStr {
    // Ensure localization is initialized for this utility
    let _ = crate::locale::setup_localization(util_name);

    let usage_label = crate::locale::translate!("common-usage");

    // Create a template that avoids clap's hardcoded {usage-heading}
    let template = format!(
        "{{before-help}}{{about-with-newline}}\n{usage_label}: {{usage}}\n\n{{all-args}}{{after-help}}"
    );

    clap::builder::StyledStr::from(template)
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
static ARGV: LazyLock<Vec<OsString>> = LazyLock::new(|| wild::args_os().collect());

static UTIL_NAME: LazyLock<String> = LazyLock::new(|| {
    let base_index = usize::from(get_utility_is_second_arg());
    let is_man = usize::from(ARGV[base_index].eq("manpage"));
    let argv_index = base_index + is_man;

    ARGV[argv_index].to_string_lossy().into_owned()
});

/// Derive the utility name.
pub fn util_name() -> &'static str {
    &UTIL_NAME
}

static EXECUTION_PHRASE: LazyLock<String> = LazyLock::new(|| {
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

#[derive(Debug)]
pub struct NonUtf8OsStrError {
    input_lossy_string: String,
}

impl std::fmt::Display for NonUtf8OsStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use os_display::Quotable;
        let quoted = self.input_lossy_string.quote();
        f.write_fmt(format_args!(
            "invalid UTF-8 input {quoted} encountered when converting to bytes on a platform that doesn't expose byte arguments",
        ))
    }
}

impl std::error::Error for NonUtf8OsStrError {}
impl error::UError for NonUtf8OsStrError {}

/// Converts an `OsStr` to a UTF-8 `&[u8]`.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the string can't be coerced to UTF-8.
pub fn os_str_as_bytes(os_string: &OsStr) -> Result<&[u8], NonUtf8OsStrError> {
    #[cfg(unix)]
    return Ok(os_string.as_bytes());

    #[cfg(not(unix))]
    os_string
        .to_str()
        .ok_or_else(|| NonUtf8OsStrError {
            input_lossy_string: os_string.to_string_lossy().into_owned(),
        })
        .map(|s| s.as_bytes())
}

/// Performs a potentially lossy conversion from `OsStr` to UTF-8 bytes.
///
/// This is always lossless on unix platforms,
/// and wraps [`OsStr::to_string_lossy`] on non-unix platforms.
pub fn os_str_as_bytes_lossy(os_string: &OsStr) -> Cow<'_, [u8]> {
    #[cfg(unix)]
    return Cow::from(os_string.as_bytes());

    #[cfg(not(unix))]
    match os_string.to_string_lossy() {
        Cow::Borrowed(slice) => Cow::from(slice.as_bytes()),
        Cow::Owned(owned) => Cow::from(owned.into_bytes()),
    }
}

/// Converts a `&[u8]` to an `&OsStr`,
/// or parses it as UTF-8 into an [`OsString`] on non-unix platforms.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the bytes can't be parsed as UTF-8.
pub fn os_str_from_bytes(bytes: &[u8]) -> mods::error::UResult<Cow<'_, OsStr>> {
    #[cfg(unix)]
    return Ok(Cow::Borrowed(OsStr::from_bytes(bytes)));

    #[cfg(not(unix))]
    Ok(Cow::Owned(OsString::from(str::from_utf8(bytes).map_err(
        |_| mods::error::UUsageError::new(1, "Unable to transform bytes into OsStr"),
    )?)))
}

/// Converts a `Vec<u8>` into an `OsString`, parsing as UTF-8 on non-unix platforms.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the bytes can't be parsed as UTF-8.
pub fn os_string_from_vec(vec: Vec<u8>) -> mods::error::UResult<OsString> {
    #[cfg(unix)]
    return Ok(OsString::from_vec(vec));

    #[cfg(not(unix))]
    Ok(OsString::from(String::from_utf8(vec).map_err(|_| {
        mods::error::UUsageError::new(1, "invalid UTF-8 was detected in one or more arguments")
    })?))
}

/// Converts an `OsString` into a `Vec<u8>`, parsing as UTF-8 on non-unix platforms.
///
/// This always succeeds on unix platforms,
/// and fails on other platforms if the bytes can't be parsed as UTF-8.
pub fn os_string_to_vec(s: OsString) -> mods::error::UResult<Vec<u8>> {
    #[cfg(unix)]
    let v = s.into_vec();
    #[cfg(not(unix))]
    let v = s
        .into_string()
        .map_err(|_| {
            mods::error::UUsageError::new(1, "invalid UTF-8 was detected in one or more arguments")
        })?
        .into();

    Ok(v)
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
/// prompt_yes!("Do you want to delete '{file}'?");
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

/// Represent either a character or a byte.
/// Used to iterate on partially valid UTF-8 data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharByte {
    Char(char),
    Byte(u8),
}

impl From<char> for CharByte {
    fn from(value: char) -> Self {
        CharByte::Char(value)
    }
}

impl From<u8> for CharByte {
    fn from(value: u8) -> Self {
        CharByte::Byte(value)
    }
}

impl From<&u8> for CharByte {
    fn from(value: &u8) -> Self {
        CharByte::Byte(*value)
    }
}

struct Utf8ChunkIterator<'a> {
    iter: Box<dyn Iterator<Item = CharByte> + 'a>,
}

impl Iterator for Utf8ChunkIterator<'_> {
    type Item = CharByte;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> From<Utf8Chunk<'a>> for Utf8ChunkIterator<'a> {
    fn from(chk: Utf8Chunk<'a>) -> Utf8ChunkIterator<'a> {
        Self {
            iter: Box::new(
                chk.valid()
                    .chars()
                    .map(CharByte::from)
                    .chain(chk.invalid().iter().map(CharByte::from)),
            ),
        }
    }
}

/// Iterates on the valid and invalid parts of a byte sequence with regard to
/// the UTF-8 encoding.
pub struct CharByteIterator<'a> {
    iter: Box<dyn Iterator<Item = CharByte> + 'a>,
}

impl<'a> CharByteIterator<'a> {
    /// Make a `CharByteIterator` from a byte slice.
    /// [`CharByteIterator`]
    pub fn new(input: &'a [u8]) -> CharByteIterator<'a> {
        Self {
            iter: Box::new(input.utf8_chunks().flat_map(Utf8ChunkIterator::from)),
        }
    }
}

impl Iterator for CharByteIterator<'_> {
    type Item = CharByte;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub trait IntoCharByteIterator<'a> {
    fn iter_char_bytes(self) -> CharByteIterator<'a>;
}

impl<'a> IntoCharByteIterator<'a> for &'a [u8] {
    fn iter_char_bytes(self) -> CharByteIterator<'a> {
        CharByteIterator::new(self)
    }
}

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
