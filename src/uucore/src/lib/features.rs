// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// features ~ feature-gated modules (core/bundler file)
//
// spell-checker:ignore (features) extendedbigdecimal logind

#[cfg(feature = "backup-control")]
pub mod backup_control;
#[cfg(feature = "buf-copy")]
pub mod buf_copy;
#[cfg(feature = "checksum")]
pub mod checksum;
#[cfg(feature = "colors")]
pub mod colors;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "extendedbigdecimal")]
pub mod extendedbigdecimal;
#[cfg(feature = "fast-inc")]
pub mod fast_inc;
#[cfg(feature = "format")]
pub mod format;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "fsext")]
pub mod fsext;
#[cfg(feature = "i18n-common")]
pub mod i18n;
#[cfg(feature = "lines")]
pub mod lines;
#[cfg(feature = "parser")]
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
#[cfg(all(unix, any(feature = "pipes", feature = "buf-copy")))]
pub mod pipes;
#[cfg(all(target_os = "linux", feature = "proc-info"))]
pub mod proc_info;
#[cfg(all(unix, feature = "process"))]
pub mod process;
#[cfg(all(target_os = "linux", feature = "tty"))]
pub mod tty;

#[cfg(all(unix, feature = "fsxattr"))]
pub mod fsxattr;
#[cfg(all(target_os = "linux", feature = "selinux"))]
pub mod selinux;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;
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
