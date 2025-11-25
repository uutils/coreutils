// spell-checker:ignore (ToDO) getusername

#[cfg(unix)]
pub use self::unix::get_username;

#[cfg(windows)]
pub use self::windows::get_username;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
