/// Utilities for printing paths, with special attention paid to special
/// characters and invalid unicode.
///
/// For displaying paths in informational messages use `Quotable::quote`. This
/// will wrap quotes around the filename and add the necessary escapes to make
/// it copy/paste-able into a shell.
///
/// # Examples
/// ```
/// use std::path::Path;
/// use uucore::display::{Quotable, println_verbatim};
///
/// let path = Path::new("foo/bar.baz");
///
/// println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
/// # Ok::<(), std::io::Error>(())
/// ```
// spell-checker:ignore Fbar
use std::ffi::OsStr;
#[cfg(any(unix, target_os = "wasi", windows))]
use std::fmt::Write as FmtWrite;
use std::fmt::{self, Display, Formatter};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;
#[cfg(any(unix, target_os = "wasi"))]
use std::str::from_utf8;

/// An extension trait for displaying filenames to users.
pub trait Quotable {
    /// Returns an object that implements [`Display`] for printing filenames with
    /// proper quoting and escaping for the platform.
    ///
    /// On Unix this corresponds to sh/bash syntax, on Windows Powershell syntax
    /// is used.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use uucore::display::Quotable;
    ///
    /// let path = Path::new("foo/bar.baz");
    ///
    /// println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
    /// ```
    fn quote(&self) -> Quoted<'_>;
}

impl<T> Quotable for T
where
    T: AsRef<OsStr>,
{
    fn quote(&self) -> Quoted<'_> {
        Quoted(self.as_ref())
    }
}

/// A wrapper around [`OsStr`] for printing paths with quoting and escaping applied.
#[derive(Debug)]
pub struct Quoted<'a>(&'a OsStr);

impl Display for Quoted<'_> {
    #[cfg(any(unix, target_os = "wasi"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = self.0.as_bytes();

        let mut is_single_safe = true;
        let mut is_double_safe = true;
        for &ch in text {
            match ch {
                ch if ch.is_ascii_control() => return write_escaped(f, text),
                b'\'' => is_single_safe = false,
                // Unsafe characters according to:
                // https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_02_03
                b'"' | b'`' | b'$' | b'\\' => is_double_safe = false,
                _ => (),
            }
        }
        let text = match from_utf8(text) {
            Err(_) => return write_escaped(f, text),
            Ok(text) => text,
        };
        if is_single_safe {
            return write_simple(f, text, '\'');
        } else if is_double_safe {
            return write_simple(f, text, '\"');
        } else {
            return write_single_escaped(f, text);
        }

        fn write_simple(f: &mut Formatter<'_>, text: &str, quote: char) -> fmt::Result {
            f.write_char(quote)?;
            f.write_str(text)?;
            f.write_char(quote)?;
            Ok(())
        }

        fn write_single_escaped(f: &mut Formatter<'_>, text: &str) -> fmt::Result {
            let mut iter = text.split('\'');
            if let Some(chunk) = iter.next() {
                if !chunk.is_empty() {
                    write_simple(f, chunk, '\'')?;
                }
            }
            for chunk in iter {
                f.write_str("\\'")?;
                if !chunk.is_empty() {
                    write_simple(f, chunk, '\'')?;
                }
            }
            Ok(())
        }

        /// Write using the syntax described here:
        /// https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html
        ///
        /// Supported by these shells:
        /// - bash
        /// - zsh
        /// - busybox sh
        /// - mksh
        ///
        /// Not supported by these:
        /// - fish
        /// - dash
        /// - tcsh
        fn write_escaped(f: &mut Formatter<'_>, text: &[u8]) -> fmt::Result {
            f.write_str("$'")?;
            for chunk in from_utf8_iter(text) {
                match chunk {
                    Ok(chunk) => {
                        for ch in chunk.chars() {
                            match ch {
                                '\n' => f.write_str("\\n")?,
                                '\t' => f.write_str("\\t")?,
                                '\r' => f.write_str("\\r")?,
                                // We could do \b, \f, \v, etc., but those are
                                // rare enough to be confusing.
                                // \0 doesn't work consistently because of the
                                // octal \nnn syntax, and null bytes can't appear
                                // in filenames anyway.
                                ch if ch.is_ascii_control() => write!(f, "\\x{:02X}", ch as u8)?,
                                '\\' | '\'' => {
                                    // '?' and '"' can also be escaped this way
                                    // but AFAICT there's no reason to do so
                                    f.write_char('\\')?;
                                    f.write_char(ch)?;
                                }
                                ch => {
                                    f.write_char(ch)?;
                                }
                            }
                        }
                    }
                    Err(unit) => write!(f, "\\x{:02X}", unit)?,
                }
            }
            f.write_char('\'')?;
            Ok(())
        }
    }

    #[cfg(windows)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Behavior is based on PowerShell.
        // ` takes the role of \ since \ is already used as the path separator.
        // Things are UTF-16-oriented, so we escape code units as "`u{1234}".
        use std::char::decode_utf16;
        use std::os::windows::ffi::OsStrExt;

        // Getting the "raw" representation of an OsStr is actually expensive,
        // so avoid it if unnecessary.
        let text = match self.0.to_str() {
            None => return write_escaped(f, self.0),
            Some(text) => text,
        };

        let mut is_single_safe = true;
        let mut is_double_safe = true;
        for ch in text.chars() {
            match ch {
                ch if ch.is_ascii_control() => return write_escaped(f, self.0),
                '\'' => is_single_safe = false,
                '"' | '`' | '$' => is_double_safe = false,
                _ => (),
            }
        }

        if is_single_safe || !is_double_safe {
            return write_simple(f, text, '\'');
        } else {
            return write_simple(f, text, '"');
        }

        fn write_simple(f: &mut Formatter<'_>, text: &str, quote: char) -> fmt::Result {
            // Quotes in Powershell can be escaped by doubling them
            f.write_char(quote)?;
            let mut iter = text.split(quote);
            if let Some(chunk) = iter.next() {
                f.write_str(chunk)?;
            }
            for chunk in iter {
                f.write_char(quote)?;
                f.write_char(quote)?;
                f.write_str(chunk)?;
            }
            f.write_char(quote)?;
            Ok(())
        }

        fn write_escaped(f: &mut Formatter<'_>, text: &OsStr) -> fmt::Result {
            f.write_char('"')?;
            for ch in decode_utf16(text.encode_wide()) {
                match ch {
                    Ok(ch) => match ch {
                        '\0' => f.write_str("`0")?,
                        '\r' => f.write_str("`r")?,
                        '\n' => f.write_str("`n")?,
                        '\t' => f.write_str("`t")?,
                        ch if ch.is_ascii_control() => write!(f, "`u{{{:04X}}}", ch as u8)?,
                        '`' => f.write_str("``")?,
                        '$' => f.write_str("`$")?,
                        '"' => f.write_str("\"\"")?,
                        ch => f.write_char(ch)?,
                    },
                    Err(err) => write!(f, "`u{{{:04X}}}", err.unpaired_surrogate())?,
                }
            }
            f.write_char('"')?;
            Ok(())
        }
    }

    #[cfg(not(any(unix, target_os = "wasi", windows)))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // As a fallback, we use Rust's own escaping rules.
        // This is reasonably sane and very easy to implement.
        // We use single quotes because that's hardcoded in a lot of tests.
        write!(f, "'{}'", self.0.to_string_lossy().escape_debug())
    }
}

#[cfg(any(unix, target_os = "wasi"))]
fn from_utf8_iter(mut bytes: &[u8]) -> impl Iterator<Item = Result<&str, u8>> {
    std::iter::from_fn(move || {
        if bytes.is_empty() {
            return None;
        }
        match from_utf8(bytes) {
            Ok(text) => {
                bytes = &[];
                Some(Ok(text))
            }
            Err(err) if err.valid_up_to() == 0 => {
                let res = bytes[0];
                bytes = &bytes[1..];
                Some(Err(res))
            }
            Err(err) => {
                let (valid, rest) = bytes.split_at(err.valid_up_to());
                bytes = rest;
                Some(Ok(from_utf8(valid).unwrap()))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_quote(cases: &[(impl AsRef<OsStr>, &str)]) {
        for (case, expected) in cases {
            assert_eq!(case.quote().to_string(), *expected);
        }
    }

    /// This should hold on any platform, or else a lot of other tests will fail.
    #[test]
    fn test_basic() {
        verify_quote(&[
            ("foo", "'foo'"),
            ("", "''"),
            ("foo/bar.baz", "'foo/bar.baz'"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn test_unix() {
        verify_quote(&[
            ("can't", r#""can't""#),
            (r#"can'"t"#, r#"'can'\''"t'"#),
            (r#"can'$t"#, r#"'can'\''$t'"#),
            ("foo\nb\ta\r\\\0`r", r#"$'foo\nb\ta\r\\\x00`r'"#),
            ("foo\x02", r#"$'foo\x02'"#),
            (r#"'$''"#, r#"\''$'\'\'"#),
        ]);
        verify_quote(&[(OsStr::from_bytes(b"foo\xFF"), r#"$'foo\xFF'"#)]);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        verify_quote(&[
            (r#"foo\bar"#, r#"'foo\bar'"#),
            ("can't", r#""can't""#),
            (r#"can'"t"#, r#"'can''"t'"#),
            (r#"can'$t"#, r#"'can''$t'"#),
            ("foo\nb\ta\r\\\0`r", r#""foo`nb`ta`r\`0``r""#),
            ("foo\x02", r#""foo`u{0002}""#),
            (r#"'$''"#, r#"'''$'''''"#),
        ]);
        verify_quote(&[(
            OsString::from_wide(&[b'x' as u16, 0xD800]),
            r#""x`u{D800}""#,
        )])
    }

    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn test_utf8_iter() {
        const CASES: &[(&[u8], &[Result<&str, u8>])] = &[
            (b"", &[]),
            (b"hello", &[Ok("hello")]),
            // Immediately invalid
            (b"\xFF", &[Err(b'\xFF')]),
            // Incomplete UTF-8
            (b"\xC2", &[Err(b'\xC2')]),
            (b"\xF4\x8F", &[Err(b'\xF4'), Err(b'\x8F')]),
            (b"\xFF\xFF", &[Err(b'\xFF'), Err(b'\xFF')]),
            (b"hello\xC2", &[Ok("hello"), Err(b'\xC2')]),
            (b"\xFFhello", &[Err(b'\xFF'), Ok("hello")]),
            (b"\xFF\xC2hello", &[Err(b'\xFF'), Err(b'\xC2'), Ok("hello")]),
            (b"foo\xFFbar", &[Ok("foo"), Err(b'\xFF'), Ok("bar")]),
            (
                b"foo\xF4\x8Fbar",
                &[Ok("foo"), Err(b'\xF4'), Err(b'\x8F'), Ok("bar")],
            ),
            (
                b"foo\xFF\xC2bar",
                &[Ok("foo"), Err(b'\xFF'), Err(b'\xC2'), Ok("bar")],
            ),
        ];
        for &(case, expected) in CASES {
            assert_eq!(
                from_utf8_iter(case).collect::<Vec<_>>().as_slice(),
                expected
            );
        }
    }
}
