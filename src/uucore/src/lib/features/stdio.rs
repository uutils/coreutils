use std::io;

#[cfg(all(feature = "signals", unix, not(target_os = "fuchsia")))]
use crate::{signals, translate};

#[cfg_attr(
    not(all(feature = "signals", unix, not(target_os = "fuchsia"))),
    expect(clippy::unnecessary_wraps)
)]
fn err_if_stdin_closed() -> io::Result<()> {
    #[cfg(all(feature = "signals", unix, not(target_os = "fuchsia")))]
    if signals::stdin_was_closed() {
        return Err(io::Error::other(translate!("error-bad-file-descriptor")));
    }

    Ok(())
}

/// Wrapper around [`std::io::Stdin`] that checks if stdin was closed
/// before trying to read.
pub struct UStdin {
    inner: io::Stdin,
}

impl UStdin {
    pub fn lock(&self) -> UStdinLock<'static> {
        UStdinLock {
            inner: self.inner.lock(),
        }
    }
}

impl io::Read for UStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        err_if_stdin_closed()?;
        self.inner.read_exact(buf)
    }
}

pub struct UStdinLock<'a> {
    inner: io::StdinLock<'a>,
}

impl io::Read for UStdinLock<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        err_if_stdin_closed()?;
        self.inner.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        err_if_stdin_closed()?;
        self.inner.read_exact(buf)
    }
}

impl io::BufRead for UStdinLock<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        err_if_stdin_closed()?;
        self.inner.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

pub fn stdin() -> UStdin {
    UStdin { inner: io::stdin() }
}
