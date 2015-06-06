/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 * (c) kwantam <kwantam@gmail.com>
 *     substantially rewritten to use the stdlib BufReader trait
 *     rather than re-implementing it here.
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::{BufRead, BufReader, Read, Write};
use std::io::Result as IoResult;

#[allow(non_snake_case)]
pub mod Bytes {
    use std::io::Write;

    pub trait Select {
        fn select<W: Write>(&mut self, bytes: usize, out: Option<&mut W>) -> Selected;
    }

    #[derive(PartialEq, Eq, Debug)]
    pub enum Selected {
        NewlineFound,
        Complete(usize),
        Partial(usize),
        EndOfFile,
    }
}

#[derive(Debug)]
pub struct ByteReader<R> where R: Read {
    inner: BufReader<R>,
}

impl<R: Read> ByteReader<R> {
    pub fn new(read: R) -> ByteReader<R> {
        ByteReader {
            inner: BufReader::with_capacity(4096, read),
        }
    }
}

impl<R: Read> Read for ByteReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.inner.read(buf)
    }
}

impl<R: Read> BufRead for ByteReader<R> {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl<R: Read> ByteReader<R> {
    pub fn consume_line(&mut self) -> usize {
        let mut bytes_consumed = 0;
        let mut consume_val;

        loop {
            { // need filled_buf to go out of scope
                let filled_buf = match self.fill_buf() {
                    Ok(b) => {
                        if b.len() == 0 {
                            return bytes_consumed
                        } else {
                            b
                        }
                    },
                    Err(e) => crash!(1, "read error: {}", e),
                };

                match filled_buf.position_elem(&b'\n') {
                    Some(idx) => {
                        consume_val = idx + 1;
                        bytes_consumed += consume_val;
                        break;
                    }
                    _ => ()
                }

                consume_val = filled_buf.len();
            }

            bytes_consumed += consume_val;
            self.consume(consume_val);
        }

        self.consume(consume_val);
        return bytes_consumed;
    }
}

impl<R: Read> self::Bytes::Select for ByteReader<R> {
    fn select<W: Write>(&mut self, bytes: usize, out: Option<&mut W>) -> Bytes::Selected {
        enum SRes {
            Comp,
            Part,
            Newl,
        };

        use self::Bytes::Selected::*;

        let (res, consume_val) = {
            let buffer = match self.fill_buf() {
                Err(e) => crash!(1, "read error: {}", e),
                Ok(b) => b,
            };

            let (res, consume_val) = match buffer.len() {
                0 => return EndOfFile,
                buf_used if bytes < buf_used => {
                    // because the output delimiter should only be placed between
                    // segments check if the byte after bytes is a newline
                    let buf_slice = &buffer[0..bytes + 1];

                    match buf_slice.position_elem(&b'\n') {
                        Some(idx) => (SRes::Newl, idx+1),
                        None => (SRes::Comp, bytes),
                    }
                },
                _ => {
                    match buffer.position_elem(&b'\n') {
                        Some(idx) => (SRes::Newl, idx+1),
                        None => (SRes::Part, buffer.len()),
                    }
                },
            };

            match out {
                Some(out) => pipe_crash_if_err!(1, out.write_all(&buffer[0..consume_val])),
                None => (),
            }
            (res, consume_val)
        };

        self.consume(consume_val);
        match res {
            SRes::Comp => Complete(consume_val),
            SRes::Part => Partial(consume_val),
            SRes::Newl => NewlineFound,
        }
    }
}
