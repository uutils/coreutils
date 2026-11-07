// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::{self, Write};

/// Return the writer for the checksum output.
///
/// Rust opens `/dev/null` on a closed stdout before `main`, so a write to it
/// does not fail. On Unix, except Fuchsia, uucore records the closed stdout at
/// startup, and this writer then fails with EBADF. It has the same line buffer
/// as stdout, so the error occurs at the same point as a real write error.
pub fn stdout_writer() -> Box<dyn Write> {
    // The same targets as the startup state capture in uucore_procs.
    #[cfg(all(unix, not(target_os = "fuchsia")))]
    if crate::signals::stdout_was_closed() {
        /// A writer that fails like a closed file descriptor.
        struct ClosedStdout;

        impl Write for ClosedStdout {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(nix::errno::Errno::EBADF.into())
            }

            // With no output, a closed stdout is not an error.
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        return Box::new(io::LineWriter::new(ClosedStdout));
    }
    Box::new(io::stdout())
}
