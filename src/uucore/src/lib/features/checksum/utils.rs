// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use os_display::Quotable;
use std::{ffi::OsStr, path::Path};

/// Create a string version of the OsStr for display.
pub fn get_filename_for_output(filename: &OsStr, input_is_stdin: bool) -> String {
    if input_is_stdin {
        "standard input"
    } else {
        filename.to_str().unwrap()
    }
    .maybe_quote()
    .to_string()
}

/// Given a Path, create an escaped string version of it, where control chars
/// are preceded with `\` backslashes.
pub fn escape_filename(filename: &Path) -> (String, &'static str) {
    let original = filename.as_os_str().to_string_lossy();
    let escaped = original
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let prefix = if escaped == original { "" } else { "\\" };
    (escaped, prefix)
}

/// Revert the effect of `escape_filename`, takes a byte array in argument.
pub fn unescape_filename(filename: &[u8]) -> (Vec<u8>, &'static str) {
    let mut unescaped = Vec::with_capacity(filename.len());
    let mut byte_iter = filename.iter().peekable();
    loop {
        let Some(byte) = byte_iter.next() else {
            break;
        };
        if *byte == b'\\' {
            match byte_iter.next() {
                Some(b'\\') => unescaped.push(b'\\'),
                Some(b'n') => unescaped.push(b'\n'),
                Some(b'r') => unescaped.push(b'\r'),
                Some(x) => {
                    unescaped.push(b'\\');
                    unescaped.push(*x);
                }
                _ => {}
            }
        } else {
            unescaped.push(*byte);
        }
    }
    let prefix = if unescaped == filename { "" } else { "\\" };
    (unescaped, prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_filename() {
        let (escaped, prefix) = escape_filename(Path::new("testfile.txt"));
        assert_eq!(escaped, "testfile.txt");
        assert_eq!(prefix, "");

        let (escaped, prefix) = escape_filename(Path::new("test\nfile.txt"));
        assert_eq!(escaped, "test\\nfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(Path::new("test\rfile.txt"));
        assert_eq!(escaped, "test\\rfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(Path::new("test\\file.txt"));
        assert_eq!(escaped, "test\\\\file.txt");
        assert_eq!(prefix, "\\");
    }

    #[test]
    fn test_unescape_filename() {
        let (unescaped, prefix) = unescape_filename(b"test\\nfile.txt");
        assert_eq!(unescaped, b"test\nfile.txt");
        assert_eq!(prefix, "\\");
        let (unescaped, prefix) = unescape_filename(b"test\\nfile.txt");
        assert_eq!(unescaped, b"test\nfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename(b"test\\rfile.txt");
        assert_eq!(unescaped, b"test\rfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename(b"test\\\\file.txt");
        assert_eq!(unescaped, b"test\\file.txt");
        assert_eq!(prefix, "\\");
    }
}
