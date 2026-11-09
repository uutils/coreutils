// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::{self, Write};

// it's possible that using a smaller or larger buffer might provide better performance
pub(super) const BUF_SIZE: usize = 16 * 1024;

pub fn exec(bytes: &[u8]) -> io::Result<()> {
    #[cfg(not(unix))]
    let mut stdout = io::stdout().lock();
    // catch `EBADF` when stdout is read-only.
    #[cfg(unix)]
    let mut stdout = uucore::io::RawWriter(rustix::stdio::stdout());
    loop {
        stdout.write_all(bytes)?;
    }
}
