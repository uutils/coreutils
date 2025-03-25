use std::io::BufRead;

use uucore::error::{FromIo, UResult, USimpleError};
use uucore::translate;

/// A uniform integer generator that tries to exactly match GNU shuf's --random-source.
///
/// It's not particularly efficient and possibly not quite uniform. It should *only* be
/// used for compatibility with GNU: other modes shouldn't touch this code.
///
/// All the logic here was black box reverse engineered. It might not match up in all edge
/// cases but it gives identical results on many different large and small inputs.
///
/// It seems that GNU uses fairly textbook rejection sampling to generate integers, reading
/// one byte at a time until it has enough entropy, and recycling leftover entropy after
/// accepting or rejecting a value.
///
/// To do your own experiments, start with commands like these:
///
///   printf '\x01\x02\x03\x04' | shuf -i0-255 -r --random-source=/dev/stdin
///
/// Then vary the integer range and the input and the input length. It can be useful to
/// see when exactly shuf crashes with an "end of file" error.
///
/// To spot small inconsistencies it's useful to run:
///
///   diff -y <(my_shuf ...) <(shuf -i0-{MAX} -r --random-source={INPUT}) | head -n 50
pub struct RandomSourceAdapter<R> {
    reader: R,
    state: u64,
    entropy: u64,
}

impl<R> RandomSourceAdapter<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: 0,
            entropy: 0,
        }
    }
}

impl<R: BufRead> RandomSourceAdapter<R> {
    pub fn get_value(&mut self, at_most: u64) -> UResult<u64> {
        while self.entropy < at_most {
            let buf = self
                .reader
                .fill_buf()
                .map_err_context(|| translate!("shuf-error-read-random-bytes"))?;
            let Some(&byte) = buf.first() else {
                return Err(USimpleError::new(
                    1,
                    translate!("shuf-error-end-of-random-bytes"),
                ));
            };
            self.reader.consume(1);
            // Is overflow OK here? Won't it cause bias? (Seems to work out...)
            self.state = self.state.wrapping_mul(256).wrapping_add(byte as u64);
            self.entropy = self.entropy.wrapping_mul(256).wrapping_add(255);
        }

        if at_most == u64::MAX {
            // at_most + 1 would overflow but this case is easy.
            let val = self.state;
            self.entropy = 0;
            self.state = 0;
            return Ok(val);
        }

        let num_possibilities = at_most + 1;

        // If the generated number falls within this margin at the upper end of the
        // range then we retry to avoid modulo bias.
        let margin = ((self.entropy as u128 + 1) % num_possibilities as u128) as u64;
        let safe_zone = self.entropy - margin;

        if self.state <= safe_zone {
            let val = self.state % num_possibilities;
            // Reuse the rest of the state.
            self.state /= num_possibilities;
            // We need this subtraction, otherwise we consume new input slightly more
            // slowly than GNU. Not sure if it checks out mathematically.
            self.entropy -= at_most;
            self.entropy /= num_possibilities;
            Ok(val)
        } else {
            self.state %= num_possibilities;
            self.entropy %= num_possibilities;
            // I sure hope the compiler optimizes this tail call.
            self.get_value(at_most)
        }
    }

    pub fn shuffle<'a, T>(&mut self, vals: &'a mut [T], amount: usize) -> UResult<&'a mut [T]> {
        // Fisher-Yates shuffle.
        // TODO: GNU does something different if amount <= vals.len() and the input is stdin.
        // The order changes completely and depends on --head-count.
        // No clue what they might do differently and why.
        let amount = amount.min(vals.len());
        for idx in 0..amount {
            let other_idx = self.get_value((vals.len() - idx - 1) as u64)? as usize + idx;
            vals.swap(idx, other_idx);
        }
        Ok(&mut vals[..amount])
    }
}
