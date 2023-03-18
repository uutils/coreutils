// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore wstat towrite cdefg bufferedoutput
//! Buffer partial output blocks until they are completed.
//!
//! Use the [`BufferedOutput`] struct to create a buffered form of the
//! [`Output`] writer.
use crate::{Output, WriteStat};

/// Buffer partial output blocks until they are completed.
///
/// Complete blocks are written immediately to the inner [`Output`],
/// but partial blocks are stored in an internal buffer until they are
/// completed.
pub(crate) struct BufferedOutput<'a> {
    /// The unbuffered inner block writer.
    inner: Output<'a>,

    /// The internal buffer that stores a partial block.
    ///
    /// The size of this buffer is always less than the output block
    /// size (that is, the value of the `obs` command-line option).
    buf: Vec<u8>,
}

impl<'a> BufferedOutput<'a> {
    /// Add partial block buffering to the given block writer.
    ///
    /// The internal buffer size is at most the value of `obs` as
    /// defined in `inner`.
    pub(crate) fn new(inner: Output<'a>) -> Self {
        let obs = inner.settings.obs;
        Self {
            inner,
            buf: Vec::with_capacity(obs),
        }
    }

    pub(crate) fn discard_cache(&self, offset: libc::off_t, len: libc::off_t) {
        self.inner.discard_cache(offset, len);
    }

    /// Flush the partial block stored in the internal buffer.
    pub(crate) fn flush(&mut self) -> std::io::Result<WriteStat> {
        let wstat = self.inner.write_blocks(&self.buf)?;
        let n = wstat.bytes_total;
        for _ in 0..n {
            self.buf.remove(0);
        }
        Ok(wstat)
    }

    /// Synchronize the inner block writer.
    pub(crate) fn sync(&mut self) -> std::io::Result<()> {
        self.inner.sync()
    }

    /// Truncate the underlying file to the current stream position, if possible.
    pub(crate) fn truncate(&mut self) -> std::io::Result<()> {
        self.inner.dst.truncate()
    }

    /// Write the given bytes one block at a time.
    ///
    /// Only complete blocks will be written. Partial blocks will be
    /// buffered until enough bytes have been provided to complete a
    /// block. The returned [`WriteStat`] object will include the
    /// number of blocks written during execution of this function.
    pub(crate) fn write_blocks(&mut self, buf: &[u8]) -> std::io::Result<WriteStat> {
        // Concatenate the old partial block with the new incoming bytes.
        let towrite = [&self.buf, buf].concat();

        // Write all complete blocks to the inner block writer.
        //
        // For example, if the output block size were 3, the buffered
        // partial block were `b"ab"` and the new incoming bytes were
        // `b"cdefg"`, then we would write blocks `b"abc"` and
        // b`"def"` to the inner block writer.
        let n = towrite.len();
        let rem = n % self.inner.settings.obs;
        let wstat = self.inner.write_blocks(&towrite[..n - rem])?;
        self.buf.clear();

        // Buffer any remaining bytes as a partial block.
        //
        // Continuing the example above, the last byte `b"g"` would be
        // buffered as a partial block until the next call to
        // `write_blocks()`.
        for byte in &towrite[n - rem..] {
            self.buf.push(*byte);
        }

        Ok(wstat)
    }
}

#[cfg(unix)]
#[cfg(test)]
mod tests {
    use crate::bufferedoutput::BufferedOutput;
    use crate::{Dest, Output, Settings};

    #[test]
    fn test_buffered_output_write_blocks_empty() {
        let settings = Settings {
            obs: 3,
            ..Default::default()
        };
        let inner = Output {
            dst: Dest::Sink,
            settings: &settings,
        };
        let mut output = BufferedOutput::new(inner);
        let wstat = output.write_blocks(&[]).unwrap();
        assert_eq!(wstat.writes_complete, 0);
        assert_eq!(wstat.writes_partial, 0);
        assert_eq!(wstat.bytes_total, 0);
        assert_eq!(output.buf, vec![]);
    }

    #[test]
    fn test_buffered_output_write_blocks_partial() {
        let settings = Settings {
            obs: 3,
            ..Default::default()
        };
        let inner = Output {
            dst: Dest::Sink,
            settings: &settings,
        };
        let mut output = BufferedOutput::new(inner);
        let wstat = output.write_blocks(b"ab").unwrap();
        assert_eq!(wstat.writes_complete, 0);
        assert_eq!(wstat.writes_partial, 0);
        assert_eq!(wstat.bytes_total, 0);
        assert_eq!(output.buf, b"ab");
    }

    #[test]
    fn test_buffered_output_write_blocks_complete() {
        let settings = Settings {
            obs: 3,
            ..Default::default()
        };
        let inner = Output {
            dst: Dest::Sink,
            settings: &settings,
        };
        let mut output = BufferedOutput::new(inner);
        let wstat = output.write_blocks(b"abcd").unwrap();
        assert_eq!(wstat.writes_complete, 1);
        assert_eq!(wstat.writes_partial, 0);
        assert_eq!(wstat.bytes_total, 3);
        assert_eq!(output.buf, b"d");
    }

    #[test]
    fn test_buffered_output_write_blocks_append() {
        let settings = Settings {
            obs: 3,
            ..Default::default()
        };
        let inner = Output {
            dst: Dest::Sink,
            settings: &settings,
        };
        let mut output = BufferedOutput {
            inner,
            buf: b"ab".to_vec(),
        };
        let wstat = output.write_blocks(b"cdefg").unwrap();
        assert_eq!(wstat.writes_complete, 2);
        assert_eq!(wstat.writes_partial, 0);
        assert_eq!(wstat.bytes_total, 6);
        assert_eq!(output.buf, b"g");
    }

    #[test]
    fn test_buffered_output_flush() {
        let settings = Settings {
            obs: 10,
            ..Default::default()
        };
        let inner = Output {
            dst: Dest::Sink,
            settings: &settings,
        };
        let mut output = BufferedOutput {
            inner,
            buf: b"abc".to_vec(),
        };
        let wstat = output.flush().unwrap();
        assert_eq!(wstat.writes_complete, 0);
        assert_eq!(wstat.writes_partial, 1);
        assert_eq!(wstat.bytes_total, 3);
        assert_eq!(output.buf, vec![]);
    }
}
