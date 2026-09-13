// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use memchr::memchr3;
use std::io::{self, BufRead};

pub fn for_each_token<R, F>(mut reader: R, mut f: F) -> io::Result<()>
where
    R: BufRead,
    F: FnMut(&[u8]),
{
    let mut pending = Vec::new();

    loop {
        let buf = reader.fill_buf()?;

        if buf.is_empty() {
            if !pending.is_empty() {
                f(&pending);
            }
            return Ok(());
        }

        let mut pos = 0;

        while pos < buf.len() {
            if pending.is_empty() {
                // Skip whitespace before the next token.
                while pos < buf.len() && is_delimiter(buf[pos]) {
                    pos += 1;
                }

                if pos == buf.len() {
                    break;
                }
            }

            if let Some(i) = memchr3(b' ', b'\t', b'\n', &buf[pos..]) {
                let end = pos + i;

                if pending.is_empty() {
                    // Fast path: token is entirely inside this buffer.
                    f(&buf[pos..end]);
                } else {
                    // Complete a token that started in an earlier buffer.
                    pending.extend_from_slice(&buf[pos..end]);
                    f(&pending);
                    pending.clear();
                }

                pos = end + 1;
            } else {
                // Token continues into the next fill_buf() chunk.
                pending.extend_from_slice(&buf[pos..]);
                break;
            }
        }

        let consumed = buf.len();
        reader.consume(consumed);
    }
}

#[inline]
fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n')
}
