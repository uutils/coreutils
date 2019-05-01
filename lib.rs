extern crate wild;

pub fn args() -> impl Iterator<Item=String> {
    wild::args()
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
#[cfg(feature = "nix")]
extern crate nix;
#[cfg(all(feature = "lazy_static", target_os = "linux"))]
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "platform-info")]
extern crate platform_info;

#[macro_use]
mod macros;

#[macro_use]
pub mod coreopts;

pub mod panic;

#[cfg(feature = "fs")]
pub mod fs;
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

#[cfg(feature = "zero-copy")]
pub mod zero_copy;

#[cfg(all(windows, feature = "wide"))]
pub mod wide;
