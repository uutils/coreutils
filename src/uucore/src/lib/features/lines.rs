//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore (vars)
//! Iterate over lines, including the line ending character(s).
//!
//! This module provides the [`lines`] function, similar to the
//! [`BufRead::lines`] method. While the [`BufRead::lines`] method
//! yields [`String`] instances that do not include the line ending
//! characters (`"\n"` or `"\r\n"`), our functions yield
//! [`Vec`]<['u8']> instances that include the line ending
//! characters. This is useful if the input data does not end with a
//! newline character and you want to preserve the exact form of the
//! input data.
use std::io::BufRead;

/// Returns an iterator over the lines, including line ending characters.
///
/// This function is just like [`BufRead::lines`], but it includes the
/// line ending characters in each yielded [`String`] if the input
/// data has them. Set the `sep` parameter to the line ending
/// character; for Unix line endings, use `b'\n'`.
///
/// # Examples
///
/// Use `sep` to specify an alternate character for line endings. For
/// example, if lines are terminated by the null character `b'\0'`:
///
/// ```rust,ignore
/// use std::io::BufRead;
/// use std::io::Cursor;
///
/// let cursor = Cursor::new(b"x\0y\0z\0");
/// let mut it = lines(cursor, b'\0').map(|l| l.unwrap());
///
/// assert_eq!(it.next(), Some(Vec::from("x\0")));
/// assert_eq!(it.next(), Some(Vec::from("y\0")));
/// assert_eq!(it.next(), Some(Vec::from("z\0")));
/// assert_eq!(it.next(), None);
/// ```
///
/// If the input data does not end with a newline character (`'\n'`),
/// then the last [`String`] yielded by this iterator also does not
/// end with a newline:
///
/// ```rust,ignore
/// let cursor = Cursor::new(b"x\ny\nz");
/// let mut it = lines(cursor, b'\n').map(|l| l.unwrap());
///
/// assert_eq!(it.next(), Some(Vec::from("x\n")));
/// assert_eq!(it.next(), Some(Vec::from("y\n")));
/// assert_eq!(it.next(), Some(Vec::from("z")));
/// assert_eq!(it.next(), None);
/// ```
pub fn lines<B>(reader: B, sep: u8) -> Lines<B>
where
    B: BufRead,
{
    Lines { buf: reader, sep }
}

/// An iterator over the lines of an instance of `BufRead`.
///
/// This struct is generally created by calling [`lines`] on a `BufRead`.
/// Please see the documentation of [`lines`] for more details.
pub struct Lines<B> {
    buf: B,
    sep: u8,
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = std::io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<std::io::Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.read_until(self.sep, &mut buf) {
            Ok(0) => None,
            Ok(_n) => Some(Ok(buf)),
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lines::lines;
    use std::io::Cursor;

    #[test]
    fn test_lines() {
        let cursor = Cursor::new(b"x\ny\nz");
        let mut it = lines(cursor, b'\n').map(|l| l.unwrap());

        assert_eq!(it.next(), Some(Vec::from("x\n")));
        assert_eq!(it.next(), Some(Vec::from("y\n")));
        assert_eq!(it.next(), Some(Vec::from("z")));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_lines_zero_terminated() {
        use std::io::Cursor;

        let cursor = Cursor::new(b"x\0y\0z\0");
        let mut it = lines(cursor, b'\0').map(|l| l.unwrap());

        assert_eq!(it.next(), Some(Vec::from("x\0")));
        assert_eq!(it.next(), Some(Vec::from("y\0")));
        assert_eq!(it.next(), Some(Vec::from("z\0")));
        assert_eq!(it.next(), None);
    }
}
