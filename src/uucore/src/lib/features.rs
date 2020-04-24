// features ~ feature-gated modules (core/bundler file)

// spell-checker:ignore (uucore/uutils) coreopts libc musl utmpx uucore uutils winapi

#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "parse_time")]
pub mod parse_time;
#[cfg(feature = "zero-copy")]
pub mod zero_copy;

// * (platform-specific) feature-gated modules
// ** non-windows
#[cfg(all(not(windows), feature = "mode"))]
pub mod mode;
// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub mod entries;
#[cfg(all(unix, feature = "process"))]
pub mod process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;
#[cfg(all(
    unix,
    not(target_os = "fuchsia"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub mod utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub mod wide;
