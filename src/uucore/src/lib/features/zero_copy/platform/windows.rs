use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};

use crate::features::zero_copy::{AsRawObject, FromRawObject};

pub type RawObject = RawHandle;

impl<T: AsRawHandle> AsRawObject for T {
    fn as_raw_object(&self) -> RawObject {
        self.as_raw_handle()
    }
}

impl<T: FromRawHandle> FromRawObject for T {
    unsafe fn from_raw_object(obj: RawObject) -> Option<Self> {
        Some(T::from_raw_handle(obj))
    }
}

// TODO: see if there's some zero-copy stuff in Windows
