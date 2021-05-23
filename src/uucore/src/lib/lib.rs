// library ~ (core/bundler file)

// Copyright (C) ~ Alex Lyon <arcterus@mail.com>
// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

// * feature-gated external crates
#[cfg(all(feature = "lazy_static", target_os = "linux"))]
extern crate lazy_static;
#[cfg(feature = "nix")]
extern crate nix;
#[cfg(feature = "platform-info")]
extern crate platform_info;

// * feature-gated external crates (re-shared as public internal modules)
#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(feature = "winapi")]
pub extern crate winapi;

//## internal modules

mod macros; // crate macros (macro_rules-type; exported to `crate::...`)

mod features; // feature-gated code modules
mod mods; // core cross-platform modules

// * cross-platform modules
pub use crate::mods::coreopts;
pub use crate::mods::os;
pub use crate::mods::panic;
pub use crate::mods::ranges;

// * feature-gated modules
#[cfg(feature = "encoding")]
pub use crate::features::encoding;
#[cfg(feature = "fs")]
pub use crate::features::fs;
#[cfg(feature = "fsext")]
pub use crate::features::fsext;
#[cfg(feature = "parse_time")]
pub use crate::features::parse_time;
#[cfg(feature = "ringbuffer")]
pub use crate::features::ringbuffer;
#[cfg(feature = "zero-copy")]
pub use crate::features::zero_copy;

// * (platform-specific) feature-gated modules
// ** non-windows
#[cfg(all(not(windows), feature = "mode"))]
pub use crate::features::mode;
// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub use crate::features::entries;
#[cfg(all(unix, feature = "perms"))]
pub use crate::features::perms;
#[cfg(all(unix, feature = "process"))]
pub use crate::features::process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub use crate::features::signals;
#[cfg(all(
    unix,
    not(target_os = "fuchsia"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub use crate::features::utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub use crate::features::wide;

//## core functions

use std::ffi::OsString;

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
            Self::Complete(result) => result,
            Self::Lossy(result) => result,
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
                    let lossy_conversion = s_ret.to_string_lossy();
                    eprintln!(
                        "Input with broken encoding occured! (s = '{}') ",
                        &lossy_conversion
                    );
                    match handling {
                        InvalidEncodingHandling::Ignore => Err(String::new()),
                        InvalidEncodingHandling::ConvertLossy => Err(lossy_conversion.to_string()),
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
            .map(|s| match s.is_ok() {
                true => s.unwrap(),
                false => s.unwrap_err(),
            })
            .collect();

        match full_conversion {
            true => ConversionResult::Complete(result_vector),
            false => ConversionResult::Lossy(result_vector),
        }
    }

    /// convience function for a more slim interface
    fn collect_str_lossy(self) -> ConversionResult {
        self.collect_str(InvalidEncodingHandling::ConvertLossy)
    }
}

impl<T: Iterator<Item = OsString> + Sized> Args for T {}

// args() ...
pub fn args() -> impl Iterator<Item = String> {
    wild::args()
}

pub fn args_os() -> impl Iterator<Item = OsString> {
    wild::args_os()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn make_os_vec(os_str: &OsStr) -> Vec<OsString> {
        vec![
            OsString::from("test"),
            OsString::from("สวัสดี"),
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
