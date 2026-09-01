// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// features ~ feature-gated modules (core/bundler file)
//
// spell-checker:ignore (features) extendedbigdecimal logind

#[cfg(feature = "backup-control")]
pub mod backup_control;
#[cfg(feature = "benchmark")]
pub mod benchmark;
#[cfg(feature = "buf-copy")]
pub mod buf_copy;
pub mod char_width;
#[cfg(feature = "checksum")]
pub mod checksum;
#[cfg(feature = "colors")]
pub mod colors;
#[cfg(feature = "diagnostics")]
pub mod diagnostics;
// Without the feature, a no-op stand-in keeps the API — and its callers —
// compiling; they all fall back to their plain one-line messages.
#[cfg(not(feature = "diagnostics"))]
#[path = "features/diagnostics_stub.rs"]
pub mod diagnostics;
// The part of the diagnostics that is not a no-op without the feature: both
// `diagnostics` above re-export it rather than each carrying a copy.
mod diagnostics_boundary;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "extendedbigdecimal")]
pub mod extendedbigdecimal;
#[cfg(feature = "fast-inc")]
pub mod fast_inc;
#[cfg(feature = "format")]
pub mod format;
#[cfg(all(feature = "fs", not(target_os = "haiku")))]
pub mod fs;
#[cfg(feature = "fsext")]
pub mod fsext;
#[cfg(feature = "i18n-common")]
pub mod i18n;
#[cfg(feature = "lines")]
pub mod lines;
#[cfg(any(
    feature = "parser",
    feature = "parser-num",
    feature = "parser-size",
    feature = "parser-glob"
))]
pub mod parser;
#[cfg(feature = "quoting-style")]
pub mod quoting_style;
#[cfg(feature = "ranges")]
pub mod ranges;
#[cfg(feature = "ringbuffer")]
pub mod ringbuffer;
#[cfg(feature = "sum")]
pub mod sum;
#[cfg(feature = "time")]
pub mod time;
#[cfg(feature = "update-control")]
pub mod update_control;
#[cfg(feature = "uptime")]
pub mod uptime;
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
#[cfg(all(feature = "pipes", any(target_os = "linux", target_os = "android")))]
pub mod pipes;
#[cfg(all(target_os = "linux", feature = "proc-info"))]
pub mod proc_info;
#[cfg(all(any(unix, windows), feature = "process"))]
pub mod process;
#[cfg(all(unix, feature = "safe-copy"))]
pub mod safe_copy;
#[cfg(all(
    feature = "safe-traversal",
    unix,
    not(any(target_os = "aix", target_os = "hurd", target_os = "redox"))
))]
pub mod safe_traversal;
#[cfg(all(target_os = "linux", feature = "tty"))]
pub mod tty;

#[cfg(all(unix, feature = "fsxattr"))]
pub mod fsxattr;
#[cfg(feature = "hardware")]
pub mod hardware;
#[cfg(all(feature = "selinux", any(target_os = "linux", target_os = "android")))]
pub mod selinux;
#[cfg(all(
    feature = "signals",
    any(
        windows,
        target_vendor = "apple",
        target_os = "aix",
        target_os = "android",
        target_os = "cygwin",
        target_os = "freebsd",
        target_os = "illumos",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "solaris"
    )
))]
pub mod signals;
#[cfg(all(feature = "smack", target_os = "linux"))]
pub mod smack;
#[cfg(feature = "feat_systemd_logind")]
pub mod systemd_logind;
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_os = "fuchsia"),
    not(target_os = "openbsd"),
    not(target_os = "redox"),
    feature = "utmpx"
))]
pub mod utmpx;
// ** windows-only
#[cfg(all(windows, feature = "wide"))]
pub mod wide;
