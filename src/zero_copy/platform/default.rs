use crate::zero_copy::RawObject;

use std::io::{self, Write};

pub struct PlatformZeroCopyWriter;

impl PlatformZeroCopyWriter {
    pub unsafe fn new(_obj: RawObject) -> Result<Self, ()> {
        Err(())
    }
}

impl Write for PlatformZeroCopyWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        panic!("should never occur")
    }

    fn flush(&mut self) -> io::Result<()> {
        panic!("should never occur")
    }
}
