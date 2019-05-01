use crate::zero_copy::RawObject;

use std::io::{self, Write};

/// A "zero-copy" writer used on platforms for which we have no actual zero-copy implementation (or
/// which use standard read/write operations for zero-copy I/O).  This writer just delegates to the
/// inner writer used to create it.  Using this struct avoids going through the machinery used to
/// handle the case where a given writer does not support zero-copy on a platform.
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
