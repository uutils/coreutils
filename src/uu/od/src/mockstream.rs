// https://github.com/lazy-bitfield/rust-mockstream/pull/2

use std::io::{Cursor, Error, ErrorKind, Read, Result};

/// `FailingMockStream` mocks a stream which will fail upon read or write
///
/// # Examples
///
/// ```
/// use std::io::{Cursor, Read};
///
/// struct CountIo {}
///
/// impl CountIo {
///     fn read_data(&self, r: &mut Read) -> usize {
///         let mut count: usize = 0;
///         let mut retries = 3;
///
///         loop {
///             let mut buffer = [0; 5];
///             match r.read(&mut buffer) {
///                 Err(_) => {
///                     if retries == 0 { break; }
///                     retries -= 1;
///                 },
///                 Ok(0) => break,
///                 Ok(n) => count += n,
///             }
///         }
///         count
///     }
/// }
///
/// #[test]
/// fn test_io_retries() {
///     let mut c = Cursor::new(&b"1234"[..])
///             .chain(FailingMockStream::new(ErrorKind::Other, "Failing", 3))
///             .chain(Cursor::new(&b"5678"[..]));
///
///     let sut = CountIo {};
///     // this will fail unless read_data performs at least 3 retries on I/O errors
///     assert_eq!(8, sut.read_data(&mut c));
/// }
/// ```
#[derive(Clone)]
pub struct FailingMockStream {
    kind: ErrorKind,
    message: &'static str,
    repeat_count: i32,
}

impl FailingMockStream {
    /// Creates a FailingMockStream
    ///
    /// When `read` or `write` is called, it will return an error `repeat_count` times.
    /// `kind` and `message` can be specified to define the exact error.
    pub fn new(kind: ErrorKind, message: &'static str, repeat_count: i32) -> Self {
        Self {
            kind,
            message,
            repeat_count,
        }
    }

    fn error(&mut self) -> Result<usize> {
        if self.repeat_count == 0 {
            Ok(0)
        } else {
            if self.repeat_count > 0 {
                self.repeat_count -= 1;
            }
            Err(Error::new(self.kind, self.message))
        }
    }
}

impl Read for FailingMockStream {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        self.error()
    }
}

#[test]
fn test_failing_mock_stream_read() {
    let mut s = FailingMockStream::new(ErrorKind::BrokenPipe, "The dog ate the ethernet cable", 1);
    let mut v = [0; 4];
    let error = s.read(v.as_mut()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "The dog ate the ethernet cable");
    // after a single error, it will return Ok(0)
    assert_eq!(s.read(v.as_mut()).unwrap(), 0);
}

#[test]
fn test_failing_mock_stream_chain_interrupted() {
    let mut c = Cursor::new(&b"abcd"[..])
        .chain(FailingMockStream::new(
            ErrorKind::Interrupted,
            "Interrupted",
            5,
        ))
        .chain(Cursor::new(&b"ABCD"[..]));

    let mut v = [0; 8];
    c.read_exact(v.as_mut()).unwrap();
    assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41, 0x42, 0x43, 0x44]);
    assert_eq!(c.read(v.as_mut()).unwrap(), 0);
}
