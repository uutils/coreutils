//! Iterate over zero-terminated lines.
use std::io::BufRead;

/// The zero byte, representing the null character.
const ZERO: u8 = 0;

/// Returns an iterator over the lines of the given reader.
///
/// The iterator returned from this function will yield instances of
/// [`io::Result`]<[`Vec`]<[`u8`]>>, representing the bytes of the line
/// *including* the null character (with the possible exception of the
/// last line, which may not have one).
///
/// # Examples
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let cursor = Cursor::new(b"x\0y\0z\0");
/// let mut iter = zlines(cursor).map(|l| l.unwrap());
/// assert_eq!(iter.next(), Some(b"x\0".to_vec()));
/// assert_eq!(iter.next(), Some(b"y\0".to_vec()));
/// assert_eq!(iter.next(), Some(b"z\0".to_vec()));
/// assert_eq!(iter.next(), None);
/// ```
pub fn zlines<B>(buf: B) -> ZLines<B> {
    ZLines { buf }
}

/// An iterator over the zero-terminated lines of an instance of `BufRead`.
pub struct ZLines<B> {
    buf: B,
}

impl<B: BufRead> Iterator for ZLines<B> {
    type Item = std::io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<std::io::Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.read_until(ZERO, &mut buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buf)),
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::lines::zlines;
    use std::io::Cursor;

    #[test]
    fn test_null_terminated() {
        let cursor = Cursor::new(b"x\0y\0z\0");
        let mut iter = zlines(cursor).map(|l| l.unwrap());
        assert_eq!(iter.next(), Some(b"x\0".to_vec()));
        assert_eq!(iter.next(), Some(b"y\0".to_vec()));
        assert_eq!(iter.next(), Some(b"z\0".to_vec()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_not_null_terminated() {
        let cursor = Cursor::new(b"x\0y\0z");
        let mut iter = zlines(cursor).map(|l| l.unwrap());
        assert_eq!(iter.next(), Some(b"x\0".to_vec()));
        assert_eq!(iter.next(), Some(b"y\0".to_vec()));
        assert_eq!(iter.next(), Some(b"z".to_vec()));
        assert_eq!(iter.next(), None);
    }
}
