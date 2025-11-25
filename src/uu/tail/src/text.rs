// spell-checker:ignore (ToDO) kqueue

// Non-localized constants (system paths and technical identifiers)
pub const DASH: &str = "-";
pub const DEV_STDIN: &str = "/dev/stdin";
pub const FD0: &str = "/dev/fd/0";
pub const DEV_TTY: &str = "/dev/tty";
pub const DEV_PTMX: &str = "/dev/ptmx";

#[cfg(target_os = "linux")]
pub const BACKEND: &str = "inotify";
#[cfg(all(unix, not(target_os = "linux")))]
pub const BACKEND: &str = "kqueue";
#[cfg(target_os = "windows")]
pub const BACKEND: &str = "ReadDirectoryChanges";
