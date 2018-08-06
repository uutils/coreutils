extern crate wild;

pub fn args() -> Box<Iterator<Item=String>> {
    wild::args()
}

pub fn args_os() -> Box<Iterator<Item=std::ffi::OsString>> {
    wild::args_os()
}

#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(feature = "winapi")]
pub extern crate winapi;
#[cfg(feature = "failure")]
extern crate failure;
#[cfg(feature = "failure_derive")]
#[macro_use]
extern crate failure_derive;

#[macro_use]
mod macros;

#[macro_use]
pub mod coreopts;

pub mod panic;

#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "utf8")]
pub mod utf8;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "parse_time")]
pub mod parse_time;

#[cfg(all(not(windows), feature = "mode"))]
pub mod mode;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "utmpx"))]
pub mod utmpx;
#[cfg(all(unix, feature = "entries"))]
pub mod entries;
#[cfg(all(unix, feature = "process"))]
pub mod process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;

#[cfg(all(windows, feature = "wide"))]
pub mod wide;
