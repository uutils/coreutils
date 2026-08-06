// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore mockstream abcdefgh bcdefgh multifile

use std::io;
use std::io::Read;

use crate::multifile_reader::HasError;

/// Wrapper for `std::io::Read` which limits the returned bytes to a particular
/// number of bytes. Skipping leading bytes is handled upstream by
/// `MultifileReader::skip`, which can seek seekable inputs.
pub struct PartialReader<R> {
    inner: R,
    limit: Option<u64>,
}

impl<R> PartialReader<R> {
    /// Create a new `PartialReader` wrapping `inner`, limiting the output to
    /// `limit` bytes. Set `limit` to `None` if there should be no limit.
    pub fn new(inner: R, limit: Option<u64>) -> Self {
        Self { inner, limit }
    }
}

impl<R: Read> Read for PartialReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        match self.limit {
            None => loop {
                match self.inner.read(out) {
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    result => return result,
                }
            },
            Some(0) => Ok(0),
            Some(ref mut limit) => {
                let slice = if *limit > (out.len() as u64) {
                    out
                } else {
                    &mut out[0..(*limit as usize)]
                };
                loop {
                    match self.inner.read(slice) {
                        Ok(r) => {
                            *limit -= r as u64;
                            return Ok(r);
                        }
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }
}

impl<R: HasError> HasError for PartialReader<R> {
    fn has_error(&self) -> bool {
        self.inner.has_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mockstream::*;
    use std::io::{Cursor, ErrorKind, Read};

    #[test]
    fn test_read_without_limits() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), None);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_read_without_limits_with_error() {
        let mut v = [0; 10];
        let f = FailingMockStream::new(ErrorKind::PermissionDenied, "No access", 3);
        let mut sut = PartialReader::new(f, None);

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "No access");
    }

    #[test]
    fn test_read_limiting_all() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), Some(0));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }

    #[test]
    fn test_read_limiting() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), Some(6));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 6);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0, 0, 0, 0]);
    }

    #[test]
    fn test_read_limiting_with_error() {
        let mut v = [0; 10];
        let f = FailingMockStream::new(ErrorKind::PermissionDenied, "No access", 3);
        let mut sut = PartialReader::new(f, Some(6));

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "No access");
    }

    #[test]
    fn test_read_limiting_with_large_limit() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), Some(20));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_read_limiting_with_multiple_reads() {
        let mut v = [0; 3];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), Some(6));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x61, 0x62, 0x63]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x64, 0x65, 0x66]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }
}
