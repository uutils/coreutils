// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Utilities for printing paths, with special attention paid to special
//! characters and invalid unicode.
//!
//! For displaying paths in informational messages use `Quotable::quote`. This
//! will wrap quotes around the filename and add the necessary escapes to make
//! it copy/paste-able into a shell.
//!
//! For writing raw paths to stdout when the output should not be quoted or escaped,
//! use `println_verbatim`. This will preserve invalid unicode.
//!
//! # Examples
//! ```rust
//! use std::path::Path;
//! use uucore::display::{Quotable, println_verbatim};
//!
//! let path = Path::new("foo/bar.baz");
//!
//! println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
//! println_verbatim(path)?; // Prints "foo/bar.baz"
//! # Ok::<(), std::io::Error>(())
//! ```

use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Stdout, StdoutLock, Write as IoWrite};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;

// These used to be defined here, but they live in their own crate now.
pub use os_display::{Quotable, Quoted};

/// Print a path (or `OsStr`-like object) directly to stdout, with a trailing newline,
/// without losing any information if its encoding is invalid.
///
/// This function is appropriate for commands where printing paths is the point and the
/// output is likely to be captured, like `pwd` and `basename`. For informational output
/// use `Quotable::quote`.
///
/// FIXME: Invalid Unicode will produce an error on Windows. That could be fixed by
/// using low-level library calls and bypassing `io::Write`. This is not a big priority
/// because broken filenames are much rarer on Windows than on Unix.
pub fn println_verbatim<S: AsRef<OsStr>>(text: S) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all_os(text.as_ref())?;
    stdout.write_all(b"\n")?;
    Ok(())
}

/// Like `println_verbatim`, without the trailing newline.
pub fn print_verbatim<S: AsRef<OsStr>>(text: S) -> io::Result<()> {
    io::stdout().write_all_os(text.as_ref())
}

/// [`io::Write`], but for OS strings.
///
/// On Unix this works straightforwardly.
///
/// On Windows this currently returns an error if the OS string is not valid Unicode.
/// This may in the future change to allow those strings to be written to consoles.
pub trait OsWrite: io::Write {
    /// Write the entire OS string into this writer.
    ///
    /// # Errors
    ///
    /// An error is returned if the underlying I/O operation fails.
    ///
    /// On Windows, if the OS string is not valid Unicode, an error of kind
    /// [`io::ErrorKind::InvalidData`] is returned.
    fn write_all_os(&mut self, buf: &OsStr) -> io::Result<()> {
        #[cfg(any(unix, target_os = "wasi"))]
        {
            self.write_all(buf.as_bytes())
        }

        #[cfg(not(any(unix, target_os = "wasi")))]
        {
            // It's possible to write a better OsWrite impl for Windows consoles (e.g. Stdout)
            // as those are fundamentally 16-bit. If the OS string is invalid then it can be
            // encoded to 16-bit and written using raw windows_sys calls. But this is quite involved
            // (see `sys/pal/windows/stdio.rs` in the stdlib) and the value-add is small.
            //
            // There's no way to write invalid OS strings to Windows files, as those are 8-bit.

            match buf.to_str() {
                Some(text) => self.write_all(text.as_bytes()),
                // We could output replacement characters instead, but the
                // stdlib errors when sending invalid UTF-8 to the console,
                // so let's follow that.
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "OS string cannot be converted to bytes",
                )),
            }
        }
    }
}

// We do not have a blanket impl for all Write because a smarter Windows impl should
// be able to make use of AsRawHandle. Please keep this in mind when adding new impls.
impl OsWrite for File {}
impl OsWrite for Stdout {}
impl OsWrite for StdoutLock<'_> {}
// A future smarter Windows implementation can first flush the BufWriter before
// doing a raw write.
impl<W: OsWrite> OsWrite for BufWriter<W> {}

impl OsWrite for Box<dyn OsWrite> {
    fn write_all_os(&mut self, buf: &OsStr) -> io::Result<()> {
        let this: &mut dyn OsWrite = self;
        this.write_all_os(buf)
    }
}

/// Print all environment variables in the format `name=value` with the specified line ending.
///
/// This function handles non-UTF-8 environment variable names and values correctly by using
/// raw bytes on Unix systems.
pub fn print_all_env_vars<T: fmt::Display>(line_ending: T) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    for (name, value) in env::vars_os() {
        stdout.write_all_os(&name)?;
        stdout.write_all(b"=")?;
        stdout.write_all_os(&value)?;
        write!(stdout, "{line_ending}")?;
    }
    Ok(())
}
