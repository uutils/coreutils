#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::*;
#[cfg(unix)]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;

// Add any operating systems we support here
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub use self::default::*;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

// Add any operating systems we support here
#[cfg(not(any(target_os = "linux", target_os = "android")))]
mod default;
