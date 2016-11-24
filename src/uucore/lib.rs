#[cfg(feature = "libc")]
pub extern crate libc;

#[macro_use]
mod macros;

#[macro_use]
pub mod coreopts;

#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "utf8")]
pub mod utf8;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "parse_time")]
pub mod parse_time;

#[cfg(all(unix, not(target_os = "fuchsia"), feature = "utmpx"))]
pub mod utmpx;
#[cfg(all(unix, feature = "utsname"))]
pub mod utsname;
#[cfg(all(unix, feature = "entries"))]
pub mod entries;
#[cfg(all(unix, feature = "process"))]
pub mod process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;

#[cfg(all(windows, feature = "wide"))]
pub mod wide;
