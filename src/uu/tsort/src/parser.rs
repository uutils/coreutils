// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use memchr::memchr3_iter;
use std::io::{self, BufRead};

#[inline(always)]
/// Reads whitespace-separated tokens and passes each token to `f`.
///
/// The input is processed a buffer at a time because `tsort` reads an
/// unbounded stream of whitespace-separated tokens.
pub fn for_each_token<R, F>(mut reader: R, mut f: F) -> io::Result<()>
where
    R: BufRead,
    F: FnMut(&[u8]),
{
    // Holds a partial token when the current buffer ends before its delimiter.
    let mut pending = Vec::new();

    loop {
        // Keep the borrow scoped for later call to consume.
        let consumed = {
            let buf = reader.fill_buf()?;
            if buf.is_empty() {
                // EOF => process any pending token.
                if !pending.is_empty() {
                    f(&pending);
                }
                return Ok(());
            }

            let mut start = 0;
            // Find each delimiter in this Buf chunk. The bytes between `start` and
            // a delimiter form one complete token.
            for end in memchr3_iter(b' ', b'\t', b'\n', buf) {
                if !pending.is_empty() {
                    // This token started in the previous chunk and ends here.
                    // try_reserve first to report the allocation failure.
                    pending.try_reserve(end - start)?;
                    pending.extend_from_slice(&buf[start..end]);

                    f(&pending);
                    pending.clear();
                } else if start != end {
                    // The token is entirely in this chunk, so avoid copying it.
                    f(&buf[start..end]);
                } // else we encountered several whitespaces, keep going

                // move past the separator to the next token.
                start = end + 1;
            }

            if start != buf.len() {
                // last token of the input (has no space after it).
                pending.try_reserve(buf.len() - start)?;
                pending.extend_from_slice(&buf[start..]);
            }

            // end borrow for consumption.
            buf.len()
        };

        reader.consume(consumed);
    }
}
