mod read;

pub use read::{BufReadDecoder, BufReadDecoderError};

use std::cmp;
use std::str;

///
/// Incremental, zero-copy UTF-8 decoding with error handling
///
/// The original implemention was written by Simon Sapin in the utf-8 crate <https://crates.io/crates/utf-8>.
/// uu_wc used to depend on that crate.
/// The author archived the repository <https://github.com/SimonSapin/rust-utf8>.
/// They suggested incorporating the source directly into uu_wc <https://github.com/uutils/coreutils/issues/4289>.
///

#[derive(Debug, Copy, Clone)]
pub struct Incomplete {
    pub buffer: [u8; 4],
    pub buffer_len: u8,
}

impl Incomplete {
    pub fn empty() -> Self {
        Self {
            buffer: [0, 0, 0, 0],
            buffer_len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer_len == 0
    }

    pub fn new(bytes: &[u8]) -> Self {
        let mut buffer = [0, 0, 0, 0];
        let len = bytes.len();
        buffer[..len].copy_from_slice(bytes);
        Self {
            buffer,
            buffer_len: len as u8,
        }
    }

    fn take_buffer(&mut self) -> &[u8] {
        let len = self.buffer_len as usize;
        self.buffer_len = 0;
        &self.buffer[..len]
    }

    /// (consumed_from_input, None): not enough input
    /// (consumed_from_input, Some(Err(()))): error bytes in buffer
    /// (consumed_from_input, Some(Ok(()))): UTF-8 string in buffer
    fn try_complete_offsets(&mut self, input: &[u8]) -> (usize, Option<Result<(), ()>>) {
        let initial_buffer_len = self.buffer_len as usize;
        let copied_from_input;
        {
            let unwritten = &mut self.buffer[initial_buffer_len..];
            copied_from_input = cmp::min(unwritten.len(), input.len());
            unwritten[..copied_from_input].copy_from_slice(&input[..copied_from_input]);
        }
        let spliced = &self.buffer[..initial_buffer_len + copied_from_input];
        match str::from_utf8(spliced) {
            Ok(_) => {
                self.buffer_len = spliced.len() as u8;
                (copied_from_input, Some(Ok(())))
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                if valid_up_to > 0 {
                    let consumed = valid_up_to.checked_sub(initial_buffer_len).unwrap();
                    self.buffer_len = valid_up_to as u8;
                    (consumed, Some(Ok(())))
                } else {
                    match error.error_len() {
                        Some(invalid_sequence_length) => {
                            let consumed = invalid_sequence_length
                                .checked_sub(initial_buffer_len)
                                .unwrap();
                            self.buffer_len = invalid_sequence_length as u8;
                            (consumed, Some(Err(())))
                        }
                        None => {
                            self.buffer_len = spliced.len() as u8;
                            (copied_from_input, None)
                        }
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::io;

    struct Chunks<'a>(VecDeque<&'a [u8]>);

    impl<'a> io::Read for Chunks<'a> {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            unimplemented!()
        }
    }

    impl<'a> io::BufRead for Chunks<'a> {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Ok(*self.0.front().unwrap())
        }

        fn consume(&mut self, bytes: usize) {
            {
                let front = self.0.front_mut().unwrap();
                *front = &front[bytes..];
                if !front.is_empty() {
                    return;
                }
            }
            if self.0.len() > 1 {
                self.0.pop_front();
            }
        }
    }
}
