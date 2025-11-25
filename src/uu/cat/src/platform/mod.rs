#[cfg(unix)]
pub use self::unix::is_unsafe_overwrite;

#[cfg(windows)]
pub use self::windows::is_unsafe_overwrite;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
