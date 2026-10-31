// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Output buffering shared by utilities that write to standard output.

#[cfg(unix)]
use std::io::LineWriter;
use std::io::{self, BufWriter, Write};
#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::AsFd;

#[cfg(any(unix, target_os = "wasi"))]
use crate::io::RawWriter;

enum BufferMode {
    Default,
    #[cfg(unix)]
    Unbuffered,
    #[cfg(unix)]
    Line,
    #[cfg(unix)]
    Block(usize),
}

#[cfg(unix)]
fn requested_mode() -> BufferMode {
    let Some(value) = std::env::var_os("_STDBUF_O") else {
        return BufferMode::Default;
    };

    match value.to_str() {
        Some("0") => BufferMode::Unbuffered,
        Some("L") => BufferMode::Line,
        // Invalid values, including an empty string, use the normal buffer.
        Some(value) => value.parse().map_or(BufferMode::Default, BufferMode::Block),
        None => BufferMode::Default,
    }
}

// `stdbuf` currently uses a preload library on Unix.  Windows and WASI do not
// have a corresponding wrapper, so they keep the normal buffering behavior.
#[cfg(not(unix))]
fn requested_mode() -> BufferMode {
    BufferMode::Default
}

/// A buffered writer for stdout that honors the output mode requested by
/// `stdbuf` on supported platforms.
pub enum StdoutBuffer<W: Write> {
    #[cfg(unix)]
    Unbuffered(W),
    #[cfg(unix)]
    Line(LineWriter<W>),
    Block(BufWriter<W>),
}

impl<W: Write> StdoutBuffer<W> {
    /// Create a writer using the standard buffer capacity unless `stdbuf`
    /// requested another output mode.
    pub fn new(writer: W) -> Self {
        // Resolve the mode once so callers can keep hot output loops generic.
        match requested_mode() {
            BufferMode::Default => Self::Block(BufWriter::new(writer)),
            #[cfg(unix)]
            BufferMode::Unbuffered => Self::Unbuffered(writer),
            #[cfg(unix)]
            BufferMode::Line => Self::Line(LineWriter::new(writer)),
            #[cfg(unix)]
            BufferMode::Block(size) => Self::Block(BufWriter::with_capacity(size, writer)),
        }
    }

    /// Create a writer with `capacity` as its normal buffer capacity unless
    /// `stdbuf` requested another output mode.
    pub fn with_capacity(capacity: usize, writer: W) -> Self {
        match requested_mode() {
            BufferMode::Default => Self::Block(BufWriter::with_capacity(capacity, writer)),
            #[cfg(unix)]
            BufferMode::Unbuffered => Self::Unbuffered(writer),
            #[cfg(unix)]
            BufferMode::Line => Self::Line(LineWriter::new(writer)),
            #[cfg(unix)]
            BufferMode::Block(size) => Self::Block(BufWriter::with_capacity(size, writer)),
        }
    }

    /// Use a raw writer for the unbuffered output mode where it is available.
    #[cfg(any(unix, target_os = "wasi"))]
    pub fn into_unbuffered(writer: W) -> RawWriter<W>
    where
        W: AsFd,
    {
        RawWriter(writer)
    }

    /// Keep the underlying writer on platforms without raw descriptor I/O.
    #[cfg(not(any(unix, target_os = "wasi")))]
    pub fn into_unbuffered(writer: W) -> W {
        writer
    }
}

#[cfg(unix)]
impl<W: Write + AsFd> Write for StdoutBuffer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Unbuffered(writer) => RawWriter(&*writer).write(buf),
            Self::Line(writer) => writer.write(buf),
            Self::Block(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Unbuffered(_) => Ok(()),
            Self::Line(writer) => writer.flush(),
            Self::Block(writer) => writer.flush(),
        }
    }
}

#[cfg(not(unix))]
impl<W: Write> Write for StdoutBuffer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Block(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Block(writer) => writer.flush(),
        }
    }
}

#[cfg(unix)]
impl<W: Write + AsFd> crate::display::OsWrite for StdoutBuffer<W> {}

#[cfg(not(unix))]
impl<W: Write> crate::display::OsWrite for StdoutBuffer<W> {}
