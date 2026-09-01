// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) multifile curr fnames fname xfrd fillloop mockstream

use std::fs::File;
use std::io;
#[cfg(unix)]
use std::io::{Seek, SeekFrom};

use uucore::{display::Quotable, error::strip_errno, show_error, translate};

/// Buffer size used when skipping bytes by reading and discarding them.
const SKIP_BUFFER_SIZE: usize = 16 * 1024;

pub enum InputSource<'a> {
    FileName(&'a str),
    Stdin,
}

/// The file currently being read. A real `File` is kept as a concrete handle so
/// that `skip` can `fstat`/`seek` it; anything else (stdin, an in-memory stream)
/// can only be advanced by reading.
enum CurrentReader {
    File(File),
    #[cfg(any(unix, target_os = "wasi"))]
    Stdin(uucore::io::RawReader<rustix::fd::BorrowedFd<'static>>),
    #[cfg(not(any(unix, target_os = "wasi")))]
    Stdin(io::Stdin),
}

impl io::Read for CurrentReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
            Self::Stdin(r) => r.read(buf),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl rustix::fd::AsFd for CurrentReader {
    fn as_fd(&self) -> rustix::fd::BorrowedFd<'_> {
        match self {
            Self::File(f) => f.as_fd(),
            Self::Stdin(s) => s.0,
        }
    }
}

// MultifileReader - concatenate all our input, file or stdin.
pub struct MultifileReader<'a> {
    ni: Vec<InputSource<'a>>,
    curr_file: Option<CurrentReader>,
    any_err: bool,
    file_name: Option<&'a str>,
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
            file_name: None,
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
                        self.curr_file = Some(CurrentReader::Stdin(stdin));
                    }

                    // For non-unix platforms we don't have GNU compatibility requirements, so
                    // we don't need to prevent stdin buffering. This is sub-optimal (since
                    // there will still be additional buffering further up the stack), but
                    // doesn't seem worth worrying about at this time.
                    #[cfg(not(any(unix, target_os = "wasi")))]
                    {
                        self.curr_file = Some(CurrentReader::Stdin(io::stdin()));
                    }
                    break;
                }
                InputSource::FileName(fname) => {
                    match File::open(fname) {
                        Ok(f) => {
                            self.file_name = Some(fname);
                            // No need to wrap `f` in a BufReader - buffered reading is taken care
                            // of elsewhere.
                            self.curr_file = Some(CurrentReader::File(f));
                            break;
                        }
                        Err(e) => {
                            // If any file can't be opened,
                            // print an error at the time that the file is needed,
                            // then move to the next file
                            let error_msg = match e.kind() {
                                #[cfg(windows)]
                                io::ErrorKind::NotFound => "No such file or directory",
                                _ => &strip_errno(&e),
                            };
                            show_error!("{}: {error_msg}", fname.maybe_quote().external(true),);
                            self.any_err = true;
                        }
                    }
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
    #[cfg(unix)]
    if let CurrentReader::File(f) = curr
        && let Ok(meta) = f.metadata()
    {
        let size = meta.len();
        let blksize = uucore::fs::sane_blksize::sane_blksize_from_metadata(&meta);

        // A regular file larger than a block reports a reliable size, so we
        // can either drop the whole file or seek within it. Small or
        // proc-like files lie about their size and fall through to reading.
        let is_file = meta.is_file();
        if is_file && blksize < size {
            if size < n_skip {
                return Ok(n_skip - size);
            }
            if seek_forward(f, n_skip)? {
                return Ok(0);
            }
        } else if !is_file {
            // Seekable special files (character/block devices) can be
            // skipped past their end without error.
            if seek_forward(f, n_skip).unwrap_or(false) {
                return Ok(0);
            }
        }
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    match uucore::pipes::discard_n_bytes(&curr, n_skip as usize) {
        Ok(spliced) => Ok(n_skip - spliced as u64),
        Err(spliced) => {
            let read =
                uucore::io::read_and_discard(curr, n_skip - spliced as u64, SKIP_BUFFER_SIZE)?;
            Ok(n_skip - spliced as u64 - read)
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    Ok(n_skip - uucore::io::read_and_discard(curr, n_skip, SKIP_BUFFER_SIZE)?)
}

/// Seek `f` forward by `n` bytes. Returns `Ok(true)` if the seek happened, or
/// `Ok(false)` if `n` is too large to express as a seek offset (the caller
/// should fall back to reading and discarding).
#[cfg(unix)]
fn seek_forward(f: &mut File, n: u64) -> io::Result<bool> {
    match i64::try_from(n) {
        Ok(off) => f.seek(SeekFrom::Current(off)).map(|_| true),
        Err(_) => Ok(false),
    }
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
                                show_error!(
                                    "{}: {}",
                                    self.file_name.unwrap_or("I/O"),
                                    strip_errno(&e)
                                );
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
#[cfg(not(target_os = "wasi"))]
mod tests {
    use super::*;
    use std::io::{Read, Write as _};
    use tempfile::NamedTempFile;

    #[test]
    fn test_multi_file_reader_one_read() {
        let mut file1 = NamedTempFile::new().unwrap();
        file1.write_all(b"abcd").unwrap();
        let path1 = file1.path().to_str().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"ABCD").unwrap();
        let path2 = file2.path().to_str().unwrap();

        let inputs = vec![InputSource::FileName(path1), InputSource::FileName(path2)];
        let mut v = [0; 10];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 8);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41, 0x42, 0x43, 0x44, 0, 0]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 0);
    }

    #[test]
    fn test_multi_file_reader_two_reads() {
        let mut file1 = NamedTempFile::new().unwrap();
        file1.write_all(b"abcd").unwrap();
        let path1 = file1.path().to_str().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"ABCD").unwrap();
        let path2 = file2.path().to_str().unwrap();

        let inputs = vec![InputSource::FileName(path1), InputSource::FileName(path2)];
        let mut v = [0; 5];

        let mut sut = MultifileReader::new(inputs);

        assert_eq!(sut.read(v.as_mut()).unwrap(), 5);
        assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41]);
        assert_eq!(sut.read(v.as_mut()).unwrap(), 3);
        assert_eq!(v, [0x42, 0x43, 0x44, 0x64, 0x41]); // last two bytes are not overwritten
    }
}
