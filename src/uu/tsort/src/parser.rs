// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use memchr::memchr3_iter;
use std::io::BufRead;

use crate::error::{AllocationError, Error, ReadError};

#[inline(always)]
/// Reads whitespace-separated tokens and passes each token to `f`.
///
/// The input is processed a buffer at a time because `tsort` reads an
/// unbounded stream of whitespace-separated tokens.
pub fn for_each_token<R, F>(mut reader: R, mut f: F) -> Result<(), Error>
where
    R: BufRead,
    F: FnMut(&[u8]) -> Result<(), Error>,
{
    // Holds a partial token when the current buffer ends before its delimiter.
    let mut pending = Vec::new();

    loop {
        // Keep the buffer borrow scoped so `consume` can be called afterwards.
        let consumed = {
            let buf = reader.fill_buf().map_err(ReadError::Io)?;

            if buf.is_empty() {
                // EOF => process any pending token.
                if !pending.is_empty() {
                    f(&pending)?;
                }
                return Ok(());
            }

            let mut start = 0;

            // Find each delimiter in this buffer. The bytes between `start` and
            // a delimiter form one complete token.
            for end in memchr3_iter(b' ', b'\t', b'\n', buf) {
                if !pending.is_empty() {
                    // This token started in a previous buffer and ends here.
                    pending
                        .try_reserve(end - start)
                        .map_err(AllocationError::from)?;
                    pending.extend_from_slice(&buf[start..end]);

                    f(&pending)?;
                    pending.clear();
                } else if start != end {
                    // The token is entirely in this buffer, so avoid copying it.
                    f(&buf[start..end])?;
                }
                // Otherwise, consecutive whitespace: there is no token here.

                // Move past the separator to the next token.
                start = end + 1;
            }

            if start != buf.len() {
                // The final token in this buffer has no delimiter yet, so carry
                // it into the next `fill_buf()` chunk.
                pending
                    .try_reserve(buf.len() - start)
                    .map_err(AllocationError::from)?;
                pending.extend_from_slice(&buf[start..]);
            }

            buf.len()
        };

        reader.consume(consumed);
    }
}
