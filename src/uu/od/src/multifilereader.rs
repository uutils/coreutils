// spell-checker:ignore (ToDO) multifile curr fnames fname xfrd fillloop mockstream

use std::fs::File;
use std::io;
use std::io::BufReader;
use std::vec::Vec;

use uucore::display::Quotable;

pub enum InputSource<'a> {
    FileName(&'a str),
    Stdin,
    #[allow(dead_code)]
    Stream(Box<dyn io::Read>),
}

// MultifileReader - concatenate all our input, file or stdin.
pub struct MultifileReader<'a> {
    ni: Vec<InputSource<'a>>,
    curr_file: Option<Box<dyn io::Read>>,
    any_err: bool,
}

pub trait HasError {
    fn has_error(&self) -> bool;
}

impl<'b> MultifileReader<'b> {
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
                    self.curr_file = Some(Box::new(BufReader::new(std::io::stdin())));
                    break;
                }
                InputSource::FileName(fname) => {
                    match File::open(fname) {
                        Ok(f) => {
                            self.curr_file = Some(Box::new(BufReader::new(f)));
                            break;
                        }
                        Err(e) => {
                            // If any file can't be opened,
                            // print an error at the time that the file is needed,
                            // then move on the the next file.
                            // This matches the behavior of the original `od`
                            show_error!("{}: {}", fname.maybe_quote(), e);
                            self.any_err = true;
                        }
                    }
                }
                InputSource::Stream(s) => {
                    self.curr_file = Some(s);
                    break;
                }
            }
        }
    }
}

impl<'b> io::Read for MultifileReader<'b> {
    // Fill buf with bytes read from the list of files
    // Returns Ok(<number of bytes read>)
    // Handles io errors itself, thus always returns OK
    // Fills the provided buffer completely, unless it has run out of input.
    // If any call returns short (< buf.len()), all subsequent calls will return Ok<0>
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
                                show_error!("I/O: {}", e);
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

impl<'b> HasError for MultifileReader<'b> {
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
