use std;
use std::io::{IoResult, IoError};

pub struct BufReader<R> {
    reader: R,
    buffer: [u8, ..4096],
    start: uint,
    end: uint,  // exclusive
}

pub mod Bytes {
    pub trait Select {
        fn select<'a>(&'a mut self, bytes: uint) -> Selected<'a>;
    }

    pub enum Selected<'a> {
        NewlineFound(&'a [u8]),
        Complete(&'a [u8]),
        Partial(&'a [u8]),
        EndOfFile,
    }
}

impl<R: Reader> BufReader<R> {
    pub fn new(reader: R) -> BufReader<R> {
        let empty_buffer = unsafe {
            std::mem::uninitialized::<[u8, ..4096]>()
        };

        BufReader {
            reader: reader,
            buffer: empty_buffer,
            start: 0,
            end: 0,
        }
    }

    fn read(&mut self) -> IoResult<uint> {
        let buf_len = self.buffer.len();
        let buffer_fill = self.buffer.mut_slice(self.end, buf_len);

        match self.reader.read(buffer_fill) {
            Ok(nread) => {
                self.end += nread;
                Ok(nread)
            }
            error => error
        }
    }

    #[inline]
    fn maybe_fill_buf(&mut self) -> IoResult<uint> {
        if self.end == self.start {
            self.start = 0;
            self.end = 0;
        }

        if self.end <= 2048 { self.read() } else { Ok(0) }
    }

    pub fn consume_line(&mut self) -> uint {
        let mut bytes_consumed = 0;

        loop {
            match self.maybe_fill_buf() {
                Err(IoError { kind: std::io::EndOfFile, .. }) => (),
                Err(err) => fail!("read error: {}", err.desc),
                _ => ()
            }

            let buffer_used = self.end - self.start;

            if buffer_used == 0 { return bytes_consumed; }

            for idx in range(self.start, self.end) {
                if self.buffer[idx] == b'\n' {
                    self.start = idx + 1;
                    return bytes_consumed + idx + 1;
                }
            }

            bytes_consumed += buffer_used;

            self.start = 0;
            self.end = 0;
        }
    }
}

impl<R: Reader> Bytes::Select for BufReader<R> {
    fn select<'a>(&'a mut self, bytes: uint) -> Bytes::Selected<'a> {
        match self.maybe_fill_buf() {
            Err(IoError { kind: std::io::EndOfFile, .. }) => (),
            Err(err) => fail!("read error: {}", err.desc),
            _ => ()
        }

        let buffer_used = self.end - self.start;

        if buffer_used == 0 { return Bytes::EndOfFile; }

        let (complete, max_segment_len) = {
            if bytes < buffer_used {
                (true, bytes + 1)
            } else {
                (false, buffer_used)
            }
        };

        for idx in range(self.start, self.start + max_segment_len) {
            if self.buffer[idx] == b'\n' {
                let segment = self.buffer.slice(self.start, idx + 1);

                self.start = idx + 1;

                return Bytes::NewlineFound(segment);
            }
        }

        if complete {
            let segment = self.buffer.slice(self.start,
                                            self.start + bytes);

            self.start += bytes;
            Bytes::Complete(segment)
        } else {
            let segment = self.buffer.slice(self.start, self.end);

            self.start = 0;
            self.end = 0;
            Bytes::Partial(segment)
        }
    }
}
