// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Output buffering shared by utilities that write to standard output.

#[cfg(any(unix, windows))]
use std::io::LineWriter;
use std::io::{self, BufWriter, Write};

enum BufferMode {
    Default,
    #[cfg(any(unix, windows))]
    Unbuffered,
    #[cfg(any(unix, windows))]
    Line,
    #[cfg(any(unix, windows))]
    Block(usize),
}

#[cfg(any(unix, windows))]
fn requested_mode() -> BufferMode {
    let Some(value) = std::env::var_os("_STDBUF_O") else {
        return BufferMode::Default;
    };

    match value.to_str() {
        Some("0") => BufferMode::Unbuffered,
        Some("L") => BufferMode::Line,
        Some(value) => value.parse().map_or(BufferMode::Default, BufferMode::Block),
        None => BufferMode::Default,
    }
}

#[cfg(not(any(unix, windows)))]
fn requested_mode() -> BufferMode {
    BufferMode::Default
}

/// A buffered writer for stdout that also honors the output mode requested by
/// `stdbuf`.
pub enum StdoutBuffer<W: Write> {
    #[cfg(any(unix, windows))]
    Unbuffered(W),
    #[cfg(any(unix, windows))]
    Line(LineWriter<W>),
    Block(BufWriter<W>),
}

impl<W: Write> StdoutBuffer<W> {
    /// Create a writer using the standard buffer capacity unless `stdbuf`
    /// requested another output mode.
    pub fn new(writer: W) -> Self {
        match requested_mode() {
            BufferMode::Default => Self::Block(BufWriter::new(writer)),
            #[cfg(any(unix, windows))]
            BufferMode::Unbuffered => Self::Unbuffered(writer),
            #[cfg(any(unix, windows))]
            BufferMode::Line => Self::Line(LineWriter::new(writer)),
            #[cfg(any(unix, windows))]
            BufferMode::Block(size) => Self::Block(BufWriter::with_capacity(size, writer)),
        }
    }

    /// Create a writer with `capacity` as its normal buffer capacity unless
    /// `stdbuf` requested another output mode.
    pub fn with_capacity(capacity: usize, writer: W) -> Self {
        match requested_mode() {
            BufferMode::Default => Self::Block(BufWriter::with_capacity(capacity, writer)),
            #[cfg(any(unix, windows))]
            BufferMode::Unbuffered => Self::Unbuffered(writer),
            #[cfg(any(unix, windows))]
            BufferMode::Line => Self::Line(LineWriter::new(writer)),
            #[cfg(any(unix, windows))]
            BufferMode::Block(size) => Self::Block(BufWriter::with_capacity(size, writer)),
        }
    }
}

impl<W: Write> Write for StdoutBuffer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(any(unix, windows))]
            Self::Unbuffered(writer) => writer.write(buf),
            #[cfg(any(unix, windows))]
            Self::Line(writer) => writer.write(buf),
            Self::Block(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(any(unix, windows))]
            Self::Unbuffered(writer) => writer.flush(),
            #[cfg(any(unix, windows))]
            Self::Line(writer) => writer.flush(),
            Self::Block(writer) => writer.flush(),
        }
    }
}

impl<W: Write> crate::display::OsWrite for StdoutBuffer<W> {}
