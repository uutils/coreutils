#[cfg(not(target_os = "openbsd"))]
mod unix;
#[cfg(not(target_os = "openbsd"))]
pub use self::unix::*;

#[cfg(target_os = "openbsd")]
mod openbsd;
#[cfg(target_os = "openbsd")]
pub use self::openbsd::*;
