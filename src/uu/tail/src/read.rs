// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::{self, BufRead, Read};

/// Finish the current batch when a nonblocking input has no data available.
/// The same reader can be used again when the follow loop next checks it.
pub(super) struct ReadAvailable<R> {
    inner: R,
    nonblocking: bool,
}

impl<R> ReadAvailable<R> {
    pub(super) fn new(inner: R, nonblocking: bool) -> Self {
        Self { inner, nonblocking }
    }
}

impl<R: Read> Read for ReadAvailable<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Err(error) if self.nonblocking && error.kind() == io::ErrorKind::WouldBlock => Ok(0),
            result => result,
        }
    }
}

impl<R: BufRead> BufRead for ReadAvailable<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self.inner.fill_buf() {
            Err(error) if self.nonblocking && error.kind() == io::ErrorKind::WouldBlock => Ok(&[]),
            result => result,
        }
    }

    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor, ErrorKind};

    struct PauseOnce {
        error: Option<ErrorKind>,
        data: Cursor<&'static [u8]>,
    }

    impl PauseOnce {
        fn new(error: ErrorKind) -> Self {
            Self {
                error: Some(error),
                data: Cursor::new(b"later"),
            }
        }
    }

    impl Read for PauseOnce {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if let Some(error) = self.error.take() {
                Err(error.into())
            } else {
                self.data.read(buf)
            }
        }
    }

    #[test]
    fn read_can_resume_after_unavailable_data() {
        let mut reader = ReadAvailable::new(PauseOnce::new(ErrorKind::WouldBlock), true);
        let mut buf = [0; 8];
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(reader.read(&mut buf).unwrap(), 5);
        assert_eq!(&buf[..5], b"later");
    }

    #[test]
    fn buffered_read_can_resume_after_unavailable_data() {
        let mut reader =
            ReadAvailable::new(BufReader::new(PauseOnce::new(ErrorKind::WouldBlock)), true);
        assert_eq!(reader.fill_buf().unwrap(), b"");
        assert_eq!(reader.fill_buf().unwrap(), b"later");
        reader.consume(2);
        assert_eq!(reader.fill_buf().unwrap(), b"ter");
    }

    #[test]
    fn real_errors_and_blocking_reads_are_not_hidden() {
        for (error, nonblocking) in [
            (ErrorKind::WouldBlock, false),
            (ErrorKind::PermissionDenied, true),
            (ErrorKind::Interrupted, true),
        ] {
            let mut reader = ReadAvailable::new(PauseOnce::new(error), nonblocking);
            assert_eq!(reader.read(&mut [0; 8]).unwrap_err().kind(), error);
        }
    }
}
