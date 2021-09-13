/// Utilities for printing paths, with special attention paid to special
/// characters and invalid unicode.
///
/// For displaying paths in informational messages use `Quotable::quote`. This
/// will wrap quotes around the filename and add the necessary escapes to make
/// it copy/paste-able into a shell.
///
/// For writing raw paths to stdout when the output should not be quoted or escaped,
/// use `println_verbatim`. This will preserve invalid unicode.
///
/// # Examples
/// ```
/// use std::path::Path;
/// use uucore::display::{Quotable, println_verbatim};
///
/// let path = Path::new("foo/bar.baz");
///
/// println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
/// println_verbatim(path)?; // Prints "foo/bar.baz"
/// # Ok::<(), std::io::Error>(())
/// ```
// spell-checker:ignore Fbar
use std::borrow::Cow;
use std::ffi::OsStr;
#[cfg(any(unix, target_os = "wasi", windows))]
use std::fmt::Write as FmtWrite;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write as IoWrite};

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

    /// Like `quote()`, but don't actually add quotes unless necessary because of
    /// whitespace or special characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use uucore::display::Quotable;
    /// use uucore::show_error;
    ///
    /// let foo = Path::new("foo/bar.baz");
    /// let bar = Path::new("foo bar");
    ///
    /// show_error!("{}: Not found", foo.maybe_quote()); // Prints "util: foo/bar.baz: Not found"
    /// show_error!("{}: Not found", bar.maybe_quote()); // Prints "util: 'foo bar': Not found"
    /// ```
    fn maybe_quote(&self) -> Quoted<'_> {
        let mut quoted = self.quote();
        quoted.force_quote = false;
        quoted
    }
}

macro_rules! impl_as_ref {
    ($type: ty) => {
        impl Quotable for $type {
            fn quote(&self) -> Quoted<'_> {
                Quoted::new(self.as_ref())
            }
        }
    };
}

impl_as_ref!(str);
impl_as_ref!(&'_ str);
impl_as_ref!(String);
impl_as_ref!(std::path::Path);
impl_as_ref!(&'_ std::path::Path);
impl_as_ref!(std::path::PathBuf);
impl_as_ref!(std::path::Component<'_>);
impl_as_ref!(std::path::Components<'_>);
impl_as_ref!(std::path::Iter<'_>);
impl_as_ref!(std::ffi::OsStr);
impl_as_ref!(&'_ std::ffi::OsStr);
impl_as_ref!(std::ffi::OsString);

// Cow<'_, str> does not implement AsRef<OsStr> and this is unlikely to be fixed
// for backward compatibility reasons. Otherwise we'd use a blanket impl.
impl Quotable for Cow<'_, str> {
    fn quote(&self) -> Quoted<'_> {
        let text: &str = self.as_ref();
        Quoted::new(text.as_ref())
    }
}

impl Quotable for Cow<'_, std::path::Path> {
    fn quote(&self) -> Quoted<'_> {
        let text: &std::path::Path = self.as_ref();
        Quoted::new(text.as_ref())
    }
}

/// A wrapper around [`OsStr`] for printing paths with quoting and escaping applied.
#[derive(Debug, Copy, Clone)]
pub struct Quoted<'a> {
    text: &'a OsStr,
    force_quote: bool,
}

impl<'a> Quoted<'a> {
    fn new(text: &'a OsStr) -> Self {
        Quoted {
            text,
            force_quote: true,
        }
    }
}

impl Display for Quoted<'_> {
    #[cfg(any(windows, unix, target_os = "wasi"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // On Unix we emulate sh syntax. On Windows Powershell.
        // They're just similar enough to share some code.

        /// Characters with special meaning outside quotes.
        // https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_02
        // I don't know why % is in there, and GNU doesn't quote it either.
        // {} were used in a version elsewhere but seem unnecessary, GNU doesn't
        // quote them. They're used in function definitions but not in a way we
        // have to worry about.
        #[cfg(any(unix, target_os = "wasi"))]
        const SPECIAL_SHELL_CHARS: &[u8] = b"|&;<>()$`\\\"'*?[]=";
        // FIXME: I'm not a PowerShell wizard and don't know if this is correct.
        // I just copied the Unix version, removed \, and added ,{} based on
        // experimentation.
        // I have noticed that ~?*[] only get expanded in some contexts, so watch
        // out for that if doing your own tests.
        // Get-ChildItem seems unwilling to quote anything so it doesn't help.
        // There's the additional wrinkle that Windows has stricter requirements
        // for filenames: I've been testing using a Linux build of PowerShell, but
        // this code doesn't even compile on Linux.
        #[cfg(windows)]
        const SPECIAL_SHELL_CHARS: &[u8] = b"|&;<>()$`\"'*?[]=,{}";

        /// Characters with a special meaning at the beginning of a name.
        // ~ expands a home directory.
        // # starts a comment.
        // ! is a common extension for expanding the shell history.
        #[cfg(any(unix, target_os = "wasi"))]
        const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#', '!'];
        // Same deal as before, this is possibly incomplete.
        // A single stand-alone exclamation mark seems to have some special meaning.
        #[cfg(windows)]
        const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#', '@', '!'];

        /// Characters that are interpreted specially in a double-quoted string.
        #[cfg(any(unix, target_os = "wasi"))]
        const DOUBLE_UNSAFE: &[u8] = &[b'"', b'`', b'$', b'\\'];
        #[cfg(windows)]
        const DOUBLE_UNSAFE: &[u8] = &[b'"', b'`', b'$'];

        let text = match self.text.to_str() {
            None => return write_escaped(f, self.text),
            Some(text) => text,
        };

        let mut is_single_safe = true;
        let mut is_double_safe = true;
        let mut requires_quote = self.force_quote;

        if let Some(first) = text.chars().next() {
            if SPECIAL_SHELL_CHARS_START.contains(&first) {
                requires_quote = true;
            }
            // Unlike in Unix, quoting an argument may stop it
            // from being recognized as an option. I like that very much.
            // But we don't want to quote "-" because that's a common
            // special argument and PowerShell doesn't mind it.
            #[cfg(windows)]
            if first == '-' && text.len() > 1 {
                requires_quote = true;
            }
        } else {
            // Empty string
            requires_quote = true;
        }

        for ch in text.chars() {
            if ch.is_ascii() {
                let ch = ch as u8;
                if ch == b'\'' {
                    is_single_safe = false;
                }
                if DOUBLE_UNSAFE.contains(&ch) {
                    is_double_safe = false;
                }
                if !requires_quote && SPECIAL_SHELL_CHARS.contains(&ch) {
                    requires_quote = true;
                }
                if ch.is_ascii_control() {
                    return write_escaped(f, self.text);
                }
            }
            if !requires_quote && ch.is_whitespace() {
                // This includes unicode whitespace.
                // We maybe don't have to escape it, we don't escape other lookalike
                // characters either, but it's confusing if it goes unquoted.
                requires_quote = true;
            }
        }

        if !requires_quote {
            return f.write_str(text);
        } else if is_single_safe {
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

        #[cfg(any(unix, target_os = "wasi"))]
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
        #[cfg(any(unix, target_os = "wasi"))]
        fn write_escaped(f: &mut Formatter<'_>, text: &OsStr) -> fmt::Result {
            f.write_str("$'")?;
            for chunk in from_utf8_iter(text.as_bytes()) {
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

        #[cfg(windows)]
        fn write_single_escaped(f: &mut Formatter<'_>, text: &str) -> fmt::Result {
            // Quotes in Powershell can be escaped by doubling them
            f.write_char('\'')?;
            let mut iter = text.split('\'');
            if let Some(chunk) = iter.next() {
                f.write_str(chunk)?;
            }
            for chunk in iter {
                f.write_str("''")?;
                f.write_str(chunk)?;
            }
            f.write_char('\'')?;
            Ok(())
        }

        #[cfg(windows)]
        fn write_escaped(f: &mut Formatter<'_>, text: &OsStr) -> fmt::Result {
            // ` takes the role of \ since \ is already used as the path separator.
            // Things are UTF-16-oriented, so we escape code units as "`u{1234}".
            use std::char::decode_utf16;
            use std::os::windows::ffi::OsStrExt;

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
        let text = self.text.to_string_lossy();
        if self.force_quote || !text.chars().all(|ch| ch.is_alphanumeric() || ch == '.') {
            write!(f, "'{}'", text.escape_debug())
        } else {
            f.write_str(&text)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_quote(cases: &[(impl Quotable, &str)]) {
        for (case, expected) in cases {
            assert_eq!(case.quote().to_string(), *expected);
        }
    }

    fn verify_maybe(cases: &[(impl Quotable, &str)]) {
        for (case, expected) in cases {
            assert_eq!(case.maybe_quote().to_string(), *expected);
        }
    }

    /// This should hold on any platform, or else other tests fail.
    #[test]
    fn test_basic() {
        verify_quote(&[
            ("foo", "'foo'"),
            ("", "''"),
            ("foo/bar.baz", "'foo/bar.baz'"),
        ]);
        verify_maybe(&[
            ("foo", "foo"),
            ("", "''"),
            ("foo bar", "'foo bar'"),
            ("$foo", "'$foo'"),
            ("-", "-"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi", windows))]
    #[test]
    fn test_common() {
        verify_maybe(&[
            ("a#b", "a#b"),
            ("#ab", "'#ab'"),
            ("a~b", "a~b"),
            ("!", "'!'"),
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
        verify_maybe(&[
            ("-x", "-x"),
            ("a,b", "a,b"),
            ("a\\b", "'a\\b'"),
            ("}", ("}")),
        ]);
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
        )]);
        verify_maybe(&[
            ("-x", "'-x'"),
            ("a,b", "'a,b'"),
            ("a\\b", "a\\b"),
            ("}", "'}'"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn test_utf8_iter() {
        type ByteStr = &'static [u8];
        type Chunk = Result<&'static str, u8>;
        const CASES: &[(ByteStr, &[Chunk])] = &[
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
