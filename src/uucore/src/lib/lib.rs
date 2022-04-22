// library ~ (core/bundler file)

// Copyright (C) ~ Alex Lyon <arcterus@mail.com>
// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

// * feature-gated external crates (re-shared as public internal modules)
#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(feature = "winapi")]
pub extern crate winapi;

//## internal modules

mod features; // feature-gated code modules
mod macros; // crate macros (macro_rules-type; exported to `crate::...`)
mod mods; // core cross-platform modules
mod parser; // string parsing modules

pub use uucore_procs::*;

// * cross-platform modules
pub use crate::mods::backup_control;
pub use crate::mods::display;
pub use crate::mods::error;
pub use crate::mods::os;
pub use crate::mods::panic;
pub use crate::mods::ranges;
pub use crate::mods::version_cmp;

// * string parsing modules
pub use crate::parser::parse_size;
pub use crate::parser::parse_time;

// * feature-gated modules
#[cfg(feature = "encoding")]
pub use crate::features::encoding;
#[cfg(feature = "fs")]
pub use crate::features::fs;
#[cfg(feature = "fsext")]
pub use crate::features::fsext;
#[cfg(feature = "lines")]
pub use crate::features::lines;
#[cfg(feature = "memo")]
pub use crate::features::memo;
#[cfg(feature = "ringbuffer")]
pub use crate::features::ringbuffer;

// * (platform-specific) feature-gated modules
// ** non-windows (i.e. Unix + Fuchsia)
#[cfg(all(not(windows), feature = "mode"))]
pub use crate::features::mode;
// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub use crate::features::entries;
#[cfg(all(unix, feature = "perms"))]
pub use crate::features::perms;
#[cfg(all(unix, feature = "pipes"))]
pub use crate::features::pipes;
#[cfg(all(unix, feature = "process"))]
pub use crate::features::process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub use crate::features::signals;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "fuchsia"),
    not(target_os = "redox"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub use crate::features::utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub use crate::features::wide;

//## core functions

use std::ffi::OsString;
use std::sync::atomic::Ordering;

use once_cell::sync::Lazy;

use crate::display::Quotable;

#[macro_export]
macro_rules! bin {
    ($util:ident) => {
        pub fn main() {
            use std::io::Write;
            uucore::panic::mute_sigpipe_panic(); // suppress extraneous error output for SIGPIPE failures/panics
            let code = $util::uumain(uucore::args_os()); // execute utility code
            std::io::stdout().flush().expect("could not flush stdout"); // (defensively) flush stdout for utility prior to exit; see <https://github.com/rust-lang/rust/issues/23818>
            std::process::exit(code);
        }
    };
}

/// Generate the usage string for clap.
///
/// This function replaces all occurrences of `{}` with the execution phrase
/// and leaks the result to return a `&'static str`. It does **not** support
/// more advanced formatting features such as `{0}`.
pub fn format_usage(s: &str) -> &'static str {
    &*Box::leak(s.replace("{}", crate::execution_phrase()).into_boxed_str())
}

pub fn get_utility_is_second_arg() -> bool {
    crate::macros::UTILITY_IS_SECOND_ARG.load(Ordering::SeqCst)
}

pub fn set_utility_is_second_arg() {
    crate::macros::UTILITY_IS_SECOND_ARG.store(true, Ordering::SeqCst);
}

// args_os() can be expensive to call, it copies all of argv before iterating.
// So if we want only the first arg or so it's overkill. We cache it.
static ARGV: Lazy<Vec<OsString>> = Lazy::new(|| wild::args_os().collect());

static UTIL_NAME: Lazy<String> = Lazy::new(|| {
    if get_utility_is_second_arg() {
        &ARGV[1]
    } else {
        &ARGV[0]
    }
    .to_string_lossy()
    .into_owned()
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

pub enum InvalidEncodingHandling {
    Ignore,
    ConvertLossy,
    Panic,
}

#[must_use]
pub enum ConversionResult {
    Complete(Vec<String>),
    Lossy(Vec<String>),
}

impl ConversionResult {
    pub fn accept_any(self) -> Vec<String> {
        match self {
            Self::Complete(result) | Self::Lossy(result) => result,
        }
    }

    pub fn expect_lossy(self, msg: &str) -> Vec<String> {
        match self {
            Self::Lossy(result) => result,
            Self::Complete(_) => {
                panic!("{}", msg);
            }
        }
    }

    pub fn expect_complete(self, msg: &str) -> Vec<String> {
        match self {
            Self::Complete(result) => result,
            Self::Lossy(_) => {
                panic!("{}", msg);
            }
        }
    }
}

pub trait Args: Iterator<Item = OsString> + Sized {
    /// Converts each iterator item to a String and collects these into a vector
    /// On invalid encoding, the result will depend on the argument. This method allows to either drop entries with illegal encoding
    /// completely (```InvalidEncodingHandling::Ignore```), convert them using lossy-conversion (```InvalidEncodingHandling::Lossy```) which will
    /// result in strange strings or can chosen to panic (```InvalidEncodingHandling::Panic```).
    /// # Arguments
    /// * `handling` - This switch allows to switch the behavior, when invalid encoding is encountered
    /// # Panics
    /// * Occurs, when invalid encoding is encountered and handling is set to ```InvalidEncodingHandling::Panic```
    fn collect_str(self, handling: InvalidEncodingHandling) -> ConversionResult {
        let mut full_conversion = true;
        let result_vector: Vec<String> = self
            .map(|s| match s.into_string() {
                Ok(string) => Ok(string),
                Err(s_ret) => {
                    full_conversion = false;
                    eprintln!(
                        "Input with broken encoding occurred! (s = {}) ",
                        s_ret.quote()
                    );
                    match handling {
                        InvalidEncodingHandling::Ignore => Err(String::new()),
                        InvalidEncodingHandling::ConvertLossy => {
                            Err(s_ret.to_string_lossy().into_owned())
                        }
                        InvalidEncodingHandling::Panic => {
                            panic!("Broken encoding found but caller cannot handle it")
                        }
                    }
                }
            })
            .filter(|s| match handling {
                InvalidEncodingHandling::Ignore => s.is_ok(),
                _ => true,
            })
            .map(|s| match s {
                Ok(v) => v,
                Err(e) => e,
            })
            .collect();

        if full_conversion {
            ConversionResult::Complete(result_vector)
        } else {
            ConversionResult::Lossy(result_vector)
        }
    }

    /// convenience function for a more slim interface
    fn collect_str_lossy(self) -> ConversionResult {
        self.collect_str(InvalidEncodingHandling::ConvertLossy)
    }
}

impl<T: Iterator<Item = OsString> + Sized> Args for T {}

pub fn args_os() -> impl Iterator<Item = OsString> {
    ARGV.iter().cloned()
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

    fn collect_os_str(vec: Vec<OsString>, handling: InvalidEncodingHandling) -> ConversionResult {
        vec.into_iter().collect_str(handling)
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn test_invalid_utf8_args_lossy(os_str: &OsStr) {
        //assert our string is invalid utf8
        assert!(os_str.to_os_string().into_string().is_err());
        let test_vec = make_os_vec(os_str);
        let collected_to_str =
            collect_os_str(test_vec.clone(), InvalidEncodingHandling::ConvertLossy)
                .expect_lossy("Lossy conversion expected in this test: bad encoding entries should be converted as good as possible");
        //conservation of length - when accepting lossy conversion no arguments may be dropped
        assert_eq!(collected_to_str.len(), test_vec.len());
        //first indices identical
        for index in 0..2 {
            assert_eq!(
                collected_to_str.get(index).unwrap(),
                test_vec.get(index).unwrap().to_str().unwrap()
            );
        }
        //lossy conversion for string with illegal encoding is done
        assert_eq!(
            *collected_to_str.get(2).unwrap(),
            os_str.to_os_string().to_string_lossy()
        );
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn test_invalid_utf8_args_ignore(os_str: &OsStr) {
        //assert our string is invalid utf8
        assert!(os_str.to_os_string().into_string().is_err());
        let test_vec = make_os_vec(os_str);
        let collected_to_str = collect_os_str(test_vec.clone(), InvalidEncodingHandling::Ignore)
            .expect_lossy(
                "Lossy conversion expected in this test: bad encoding entries should be filtered",
            );
        //assert that the broken entry is filtered out
        assert_eq!(collected_to_str.len(), test_vec.len() - 1);
        //assert that the unbroken indices are converted as expected
        for index in 0..2 {
            assert_eq!(
                collected_to_str.get(index).unwrap(),
                test_vec.get(index).unwrap().to_str().unwrap()
            );
        }
    }

    #[test]
    fn valid_utf8_encoding_args() {
        //create a vector containing only correct encoding
        let test_vec = make_os_vec(&OsString::from("test2"));
        //expect complete conversion without losses, even when lossy conversion is accepted
        let _ = collect_os_str(test_vec, InvalidEncodingHandling::ConvertLossy)
            .expect_complete("Lossy conversion not expected in this test");
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
}
