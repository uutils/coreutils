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

use std::ffi::OsStr;
use std::io::{self, Write as IoWrite};

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
/// FIXME: This is lossy on Windows. It could probably be implemented using some low-level
/// API that takes UTF-16, without going through io::Write. This is not a big priority
/// because broken filenames are much rarer on Windows than on Unix.
pub fn println_verbatim<S: AsRef<OsStr>>(text: S) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    #[cfg(any(unix, target_os = "wasi"))]
    {
        stdout.write_all(text.as_ref().as_bytes())?;
        stdout.write_all(b"\n")?;
    }
    #[cfg(not(any(unix, target_os = "wasi")))]
    {
        writeln!(stdout, "{}", std::path::Path::new(text.as_ref()).display())?;
    }
    Ok(())
}

/// Like `println_verbatim`, without the trailing newline.
pub fn print_verbatim<S: AsRef<OsStr>>(text: S) -> io::Result<()> {
    let mut stdout = io::stdout();
    #[cfg(any(unix, target_os = "wasi"))]
    {
        stdout.write_all(text.as_ref().as_bytes())
    }
    #[cfg(not(any(unix, target_os = "wasi")))]
    {
        write!(stdout, "{}", std::path::Path::new(text.as_ref()).display())
    }
}
