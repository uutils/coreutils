// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// features ~ feature-gated modules (core/bundler file)

#[cfg(feature = "backup-control")]
pub mod backup_control;
#[cfg(feature = "checksum")]
pub mod checksum;
#[cfg(feature = "colors")]
pub mod colors;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "format")]
pub mod format;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "fsext")]
pub mod fsext;
#[cfg(feature = "lines")]
pub mod lines;
#[cfg(feature = "quoting-style")]
pub mod quoting_style;
#[cfg(feature = "ranges")]
pub mod ranges;
#[cfg(feature = "ringbuffer")]
pub mod ringbuffer;
#[cfg(feature = "sum")]
pub mod sum;
#[cfg(feature = "update-control")]
pub mod update_control;
#[cfg(feature = "version-cmp")]
pub mod version_cmp;

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

#[cfg(all(unix, not(target_os = "macos"), feature = "fsxattr"))]
pub mod fsxattr;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "fuchsia"),
    not(target_os = "openbsd"),
    not(target_os = "redox"),
    not(target_env = "musl"),
    feature = "utmpx"
))]
pub mod utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub mod wide;
