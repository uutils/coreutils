// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::{self, Write};
use uucore::io::RawWriter;
use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, pipe, splice, tee};

pub(super) const BUF_SIZE: usize = MAX_ROOTLESS_PIPE_SIZE;

pub fn exec(bytes: &[u8]) -> io::Result<()> {
    let stdout = rustix::stdio::stdout();
    // improve throughput
    let _ = rustix::pipe::fcntl_setpipe_size(stdout, MAX_ROOTLESS_PIPE_SIZE);
    // tee() cannot control offset. We can do tee only if original bytes.len() is multiple of PIPE_BUF,
    // but it is slower than mixing splice even it reduces syscalls...
    if let bytes_len @ ..=MAX_ROOTLESS_PIPE_SIZE = bytes.len()
        && let Ok((p_read, mut p_write)) = pipe::<true>()
        && p_write.write_all(bytes).is_ok()
        && let Ok((broker_read, broker_write)) = pipe::<true>()
        // GNU catches all strace injections for splice expect for 1st one (checking support of it)
        // do same things for tee too which is enough for actual usage
        && Ok(bytes_len) == tee(&p_read, &broker_write, MAX_ROOTLESS_PIPE_SIZE)
        && let Ok(spliced_first) = splice(&broker_read, &stdout, bytes_len)
    {
        let mut remain = bytes_len - spliced_first;
        loop {
            while remain > 0 {
                remain -= splice(&broker_read, &stdout, remain)?;
            }
            remain = tee(&p_read, &broker_write, MAX_ROOTLESS_PIPE_SIZE)?;
            debug_assert!(remain == bytes_len, "splice should cleanup pipe");
        }
    }

    // fallback
    let mut stdout = RawWriter(stdout);
    loop {
        stdout.write_all(bytes)?;
    }
}
