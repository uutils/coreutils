#[cfg(unix)]
pub use self::unix::instantiate_current_writer;

#[cfg(windows)]
pub use self::windows::instantiate_current_writer;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
