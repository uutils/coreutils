extern crate libc;
#[cfg(windows)] extern crate winapi;

#[macro_use]
mod macros;

pub mod fs;
pub mod parse_time;
pub mod utf8;
pub mod encoding;
pub mod coreopts;

#[cfg(unix)] pub mod c_types;
#[cfg(unix)] pub mod process;
#[cfg(unix)] pub mod signals;
#[cfg(unix)] pub mod utmpx;

#[cfg(windows)] pub mod wide;
