// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) multifile curr fnames fname xfrd fillloop mockstream

use std::cmp;
use std::fs::File;
use std::io;
use std::io::{Seek, SeekFrom};

use uucore::display::Quotable;
use uucore::show_error;
use uucore::translate;

/// Buffer size used when skipping bytes by reading and discarding them.
const SKIP_BUFFER_SIZE: usize = 16 * 1024;

pub enum InputSource<'a> {
    FileName(&'a str),
    Stdin,
    #[allow(dead_code)]
    Stream(Box<dyn io::Read>),
}

/// The file currently being read. A real `File` is kept as a concrete handle so
/// that `skip` can `fstat`/`seek` it; anything else (stdin, an in-memory stream)
/// can only be advanced by reading.
enum CurrentReader {
    File(File),
    Other(Box<dyn io::Read>),
}

impl io::Read for CurrentReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
            Self::Other(r) => r.read(buf),
        }
    }
}

// MultifileReader - concatenate all our input, file or stdin.
pub struct MultifileReader<'a> {
    ni: Vec<InputSource<'a>>,
    curr_file: Option<CurrentReader>,
    any_err: bool,
}

pub trait HasError {
    fn has_error(&self) -> bool;
}

impl MultifileReader<'_> {
    pub fn new(fnames: Vec<InputSource>) -> MultifileReader {
        let mut mf = MultifileReader {
            ni: fnames,
            curr_file: None, // normally this means done; call next_file()
            any_err: false,
        };
        mf.next_file();
        mf
    }

    fn next_file(&mut self) {
        // loop retries with subsequent files if err - normally 'loops' once
        loop {
            if self.ni.is_empty() {
                self.curr_file = None;
                break;
            }
            match self.ni.remove(0) {
                InputSource::Stdin => {
                    // In order to pass GNU compatibility tests, when the client passes in the
                    // `-N` flag we must not read any bytes beyond that limit. As such, we need
                    // to disable the default buffering for stdin below.
                    // For performance reasons we do still do buffered reads from stdin, but
                    // the buffering is done elsewhere and in a way that is aware of the `-N`
                    // limit.
                    #[cfg(any(unix, target_os = "wasi"))]
                    {
                        let stdin = uucore::io::RawReader(rustix::stdio::stdin());
                        self.curr_file = Some(CurrentReader::Other(Box::new(stdin)));
                    }

                    // For non-unix platforms we don't have GNU compatibility requirements, so
                    // we don't need to prevent stdin buffering. This is sub-optimal (since
                    // there will still be additional buffering further up the stack), but
                    // doesn't seem worth worrying about at this time.
                    #[cfg(not(any(unix, target_os = "wasi")))]
                    {
                        let stdin = io::stdin();
                        self.curr_file = Some(CurrentReader::Other(Box::new(stdin)));
                    }
                    break;
                }
                InputSource::FileName(fname) => {
                    match File::open(fname) {
                        Ok(f) => {
                            // No need to wrap `f` in a BufReader - buffered reading is taken care
                            // of elsewhere.
                            self.curr_file = Some(CurrentReader::File(f));
                            break;
                        }
                        Err(e) => {
                            // If any file can't be opened,
                            // print an error at the time that the file is needed,
                            // then move to the next file.
                            // This matches the behavior of the original `od`
                            // Format error without OS error code to match GNU od
                            let error_msg = match e.kind() {
                                io::ErrorKind::NotFound => "No such file or directory",
                                io::ErrorKind::PermissionDenied => "Permission denied",
                                _ => "I/O error",
                            };
                            show_error!("{}: {error_msg}", fname.maybe_quote().external(true));
                            self.any_err = true;
                        }
                    }
                }
                InputSource::Stream(s) => {
                    self.curr_file = Some(CurrentReader::Other(s));
                    break;
                }
            }
        }
    }

    /// Skip `n_skip` bytes from the start of the combined input.
    ///
    /// A real file is positioned by `seek` whenever that is safe: a regular
    /// file large enough that its reported size is trustworthy, or any seekable
    /// special file (e.g. `/dev/null`, which can be skipped past its empty end).
    /// Everything else - proc/sys files that report a bogus size, pipes, stdin -
    /// is advanced by reading and discarding. Skipping past the end of the whole
    /// input is an error, matching GNU `od`.
    pub fn skip(&mut self, mut n_skip: u64) -> io::Result<()> {
        while n_skip > 0 {
            let Some(curr) = self.curr_file.as_mut() else {
                break;
            };
            n_skip = skip_in_file(curr, n_skip)?;
            if n_skip == 0 {
                break;
            }
            // Current file is exhausted; continue skipping in the next one.
            self.next_file();
        }

        if n_skip > 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                translate!("od-error-skip-past-end"),
            ));
        }
        Ok(())
    }
}

/// Skip up to `n_skip` bytes within a single file. Returns the number of bytes
/// that still need to be skipped (0 if the skip landed inside this file, or
/// the remainder if the file ended first).
fn skip_in_file(curr: &mut CurrentReader, n_skip: u64) -> io::Result<u64> {
    if let CurrentReader::File(f) = curr {
        if let Ok(meta) = f.metadata() {
            let size = meta.len();
            #[cfg(unix)]
            let blksize = {
                use std::os::unix::fs::MetadataExt;
                meta.blksize()
            };
            // Without st_blksize we can't tell a trustworthy size from a bogus
            // one, so never take the size shortcut on those platforms.
            #[cfg(not(unix))]
            let blksize = size;

            // A regular file larger than a block reports a reliable size, so we
            // can either drop the whole file or seek within it. Small or
            // proc-like files lie about their size and fall through to reading.
            if meta.is_file() && blksize < size {
                if size < n_skip {
                    return Ok(n_skip - size);
                }
                if let Ok(off) = i64::try_from(n_skip) {
                    f.seek(SeekFrom::Current(off))?;
                    return Ok(0);
                }
            } else if !meta.is_file() {
                // Seekable special files (character/block devices) can be
                // skipped past their end without error.
                if let Ok(off) = i64::try_from(n_skip) {
                    if f.seek(SeekFrom::Current(off)).is_ok() {
                        return Ok(0);
                    }
                }
            }
        }
    }
    read_and_discard(curr, n_skip)
}

/// Advance `reader` by discarding up to `n_skip` bytes. Returns the number of
/// bytes left to skip; non-zero means the reader hit EOF first.
fn read_and_discard(reader: &mut impl io::Read, mut n_skip: u64) -> io::Result<u64> {
    let mut buf = [0u8; SKIP_BUFFER_SIZE];
    while n_skip > 0 {
        let want = cmp::min(n_skip, buf.len() as u64) as usize;
        match reader.read(&mut buf[..want]) {
            Ok(0) => break, // EOF: caller moves on to the next file.
            Ok(n) => n_skip -= n as u64,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n_skip)
}

impl io::Read for MultifileReader<'_> {
    /// Fill buf with bytes read from the list of files
    /// Returns `Ok(<number of bytes read>)`
    /// Handles io errors itself, thus always returns OK
    /// Fills the provided buffer completely, unless it has run out of input.
    /// If any call returns short (`< buf.len()`), all subsequent calls will return Ok<0>
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut xfrd = 0;
        // while buffer we are filling is not full.. May go through several files.
        'fillloop: while xfrd < buf.len() {
            match self.curr_file {
                None => break,
                Some(ref mut curr_file) => {
                    loop {
                        // stdin may return on 'return' (enter), even though the buffer isn't full.
                        xfrd += match curr_file.read(&mut buf[xfrd..]) {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(e) => {
                                show_error!("I/O: {e}");
                                self.any_err = true;
                                break;
                            }
                        };
                        if xfrd == buf.len() {
                            // transferred all that was asked for.
                            break 'fillloop;
                        }
                    }
                }
            }
            self.next_file();
        }
        Ok(xfrd)
    }
}

impl HasError for MultifileReader<'_> {
    fn has_error(&self) -> bool {
        self.any_err
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mockstream::*;
    use std::io::{Cursor, ErrorKind, Read};

    #[test]
    fn test_multi_file_reader_one_read() {
        let inputs = vec![
            InputSource::Stream(Box::new(Cursor::new(&b"abcd"[..]))),
            InputSource::Stream(Box::new(Cursor::new(&b"ABCD"[..]))),
        ];
        let mut v = [0; 10];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41, 0x42, 0x43, 0x44, 0, 0]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }

    #[test]
    fn test_multi_file_reader_two_reads() {
        let inputs = vec![
            InputSource::Stream(Box::new(Cursor::new(&b"abcd"[..]))),
            InputSource::Stream(Box::new(Cursor::new(&b"ABCD"[..]))),
        ];
        let mut v = [0; 5];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 5);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x42, 0x43, 0x44, 0x64, 0x41]); // last two bytes are not overwritten
    }

    #[test]
    fn test_multi_file_reader_read_error() {
        let c = Cursor::new(&b"1234"[..])
            .chain(FailingMockStream::new(ErrorKind::Other, "Failing", 1))
            .chain(Cursor::new(&b"5678"[..]));
        let inputs = vec![
            InputSource::Stream(Box::new(c)),
            InputSource::Stream(Box::new(Cursor::new(&b"ABCD"[..]))),
        ];
        let mut v = [0; 5];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 5);
        assert_eq!(v, [49, 50, 51, 52, 65]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [66, 67, 68, 52, 65]); // last two bytes are not overwritten

        // note: no retry on i/o error, so 5678 is missing
    }

    #[test]
    fn test_multi_file_reader_read_error_at_start() {
        let inputs = vec![
            InputSource::Stream(Box::new(FailingMockStream::new(
                ErrorKind::Other,
                "Failing",
                1,
            ))),
            InputSource::Stream(Box::new(Cursor::new(&b"abcd"[..]))),
            InputSource::Stream(Box::new(FailingMockStream::new(
                ErrorKind::Other,
                "Failing",
                1,
            ))),
            InputSource::Stream(Box::new(Cursor::new(&b"ABCD"[..]))),
            InputSource::Stream(Box::new(FailingMockStream::new(
                ErrorKind::Other,
                "Failing",
                1,
            ))),
        ];
        let mut v = [0; 5];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 5);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x42, 0x43, 0x44, 0x64, 0x41]); // last two bytes are not overwritten
    }
}
