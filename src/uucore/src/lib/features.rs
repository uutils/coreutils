// features ~ feature-gated modules (core/bundler file)

#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "fsext")]
pub mod fsext;
#[cfg(feature = "lines")]
pub mod lines;
#[cfg(feature = "memo")]
pub mod memo;
#[cfg(feature = "ringbuffer")]
pub mod ringbuffer;
#[cfg(feature = "memo")]
mod tokenize;

// * (platform-specific) feature-gated modules
// ** non-windows (i.e. Unix + Fuchsia)
#[cfg(all(not(windows), feature = "mode"))]
pub mod mode;

// ** unix-only
#[cfg(all(unix, feature = "entries"))]
pub mod entries;
#[cfg(all(unix, feature = "perms"))]
pub mod perms;
#[cfg(all(unix, feature = "pipes"))]
pub mod pipes;
#[cfg(all(unix, feature = "process"))]
pub mod process;

#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "fuchsia"),
    not(target_os = "redox"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub mod utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub mod wide;
