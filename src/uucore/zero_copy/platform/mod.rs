#[cfg(unix)]
pub use self::unix::*;
#[cfg(target_os = "linux")]
pub use self::linux::*;
#[cfg(windows)]
pub use self::windows::*;

// Add any operating systems we support here
#[cfg(not(any(target_os = "linux")))]
pub use self::default::*;

#[cfg(unix)]
mod unix;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

// Add any operating systems we support here
#[cfg(not(any(target_os = "linux")))]
mod default;
