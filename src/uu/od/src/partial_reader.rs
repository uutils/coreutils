// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore mockstream abcdefgh bcdefgh multifile

use std::cmp;
use std::io;
use std::io::Read;

use crate::multifile_reader::HasError;
use uucore::translate;

/// When a large number of bytes must be skipped, it will be read into a
/// dynamically allocated buffer. The buffer will be limited to this size.
const MAX_SKIP_BUFFER: usize = 16 * 1024;

/// Wrapper for `std::io::Read` which can skip bytes at the beginning
/// of the input, and it can limit the returned bytes to a particular
/// number of bytes.
pub struct PartialReader<R> {
    inner: R,
    skip: u64,
    limit: Option<u64>,
}

impl<R> PartialReader<R> {
    /// Create a new `PartialReader` wrapping `inner`, which will skip
    /// `skip` bytes, and limits the output to `limit` bytes. Set `limit`
    /// to `None` if there should be no limit.
    pub fn new(inner: R, skip: u64, limit: Option<u64>) -> Self {
        Self { inner, skip, limit }
    }
}

impl<R: Read> Read for PartialReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.skip > 0 {
            let mut bytes = [0; MAX_SKIP_BUFFER];

            while self.skip > 0 {
                let skip_count: usize = cmp::min(self.skip as usize, MAX_SKIP_BUFFER);

                loop {
                    match self.inner.read(&mut bytes[..skip_count]) {
                        Ok(0) => {
                            // this is an error as we still have more to skip
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                translate!("od-error-skip-past-end"),
                            ));
                        }
                        Ok(n) => {
                            self.skip -= n as u64;
                            break;
                        }
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        match self.limit {
            None => loop {
                match self.inner.read(out) {
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
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
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
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
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 0, None);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_read_without_limits_with_error() {
        let mut v = [0; 10];
        let f = FailingMockStream::new(ErrorKind::PermissionDenied, "No access", 3);
        let mut sut = PartialReader::new(f, 0, None);

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "No access");
    }

    #[test]
    fn test_read_skipping_bytes() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 2, None);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 6);
        assert_eq!(v, [0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0, 0, 0]);
    }

    #[test]
    fn test_read_skipping_all() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 20, None);

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnexpectedEof);
    }

    #[test]
    fn test_read_skipping_with_error() {
        let mut v = [0; 10];
        let f = FailingMockStream::new(ErrorKind::PermissionDenied, "No access", 3);
        let mut sut = PartialReader::new(f, 2, None);

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "No access");
    }

    #[test]
    fn test_read_skipping_with_two_reads_during_skip() {
        let mut v = [0; 10];
        let c = Cursor::new(&b"a"[..]).chain(Cursor::new(&b"bcdefgh"[..]));
        let mut sut = PartialReader::new(c, 2, None);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 6);
        assert_eq!(v, [0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0, 0, 0]);
    }

    #[test]
    fn test_read_skipping_huge_number() {
        let mut v = [0; 10];
        // test if it does not eat all memory....
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), u64::MAX, None);

        sut.read(v.as_mut()).unwrap_err();
    }

    #[test]
    fn test_read_limiting_all() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 0, Some(0));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }

    #[test]
    fn test_read_limiting() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 0, Some(6));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 6);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0, 0, 0, 0]);
    }

    #[test]
    fn test_read_limiting_with_error() {
        let mut v = [0; 10];
        let f = FailingMockStream::new(ErrorKind::PermissionDenied, "No access", 3);
        let mut sut = PartialReader::new(f, 0, Some(6));

        let error = sut.read(v.as_mut()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "No access");
    }

    #[test]
    fn test_read_limiting_with_large_limit() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 0, Some(20));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0, 0]);
    }

    #[test]
    fn test_read_limiting_with_multiple_reads() {
        let mut v = [0; 3];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 0, Some(6));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x61, 0x62, 0x63]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x64, 0x65, 0x66]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }

    #[test]
    fn test_read_skipping_and_limiting() {
        let mut v = [0; 10];
        let mut sut = PartialReader::new(Cursor::new(&b"abcdefgh"[..]), 2, Some(4));

        assert_eq!(sut.read(v.as_mut()).unwrap(), 4);
        assert_eq!(v, [0x63, 0x64, 0x65, 0x66, 0, 0, 0, 0, 0, 0]);
    }
}
