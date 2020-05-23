use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

use crate::zero_copy::{AsRawObject, FromRawObject};

pub type RawObject = RawFd;

impl<T: AsRawFd> AsRawObject for T {
    fn as_raw_object(&self) -> RawObject {
        self.as_raw_fd()
    }
}

// FIXME: check if this works right
impl<T: FromRawFd> FromRawObject for T {
    unsafe fn from_raw_object(obj: RawObject) -> Option<Self> {
        Some(T::from_raw_fd(obj))
    }
}
