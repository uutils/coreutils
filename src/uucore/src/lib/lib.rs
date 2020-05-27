// library ~ (core/bundler file)

// Copyright (C) ~ Alex Lyon <arcterus@mail.com>
// Copyright (C) ~ Roy Ivy III <rivy.dev@gmail.com>; MIT license

//## external crates

extern crate wild;

// * feature-gated external crates
#[cfg(feature = "failure")]
extern crate failure;
#[cfg(feature = "failure_derive")]
extern crate failure_derive;
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
pub use crate::mods::panic;

// * feature-gated modules
#[cfg(feature = "encoding")]
pub use crate::features::encoding;
#[cfg(feature = "fs")]
pub use crate::features::fs;
#[cfg(feature = "parse_time")]
pub use crate::features::parse_time;
#[cfg(feature = "zero-copy")]
pub use crate::features::zero_copy;

// * (platform-specific) feature-gated modules
// ** non-windows
#[cfg(all(not(windows), feature = "mode"))]
pub use crate::features::mode;
// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub use crate::features::entries;
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

// args() ...
pub fn args() -> impl Iterator<Item = String> {
    wild::args()
}
