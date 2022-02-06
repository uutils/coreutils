//! Take all but the last elements of an iterator.
use std::io::Read;

use memchr::memchr_iter;

use uucore::ringbuffer::RingBuffer;

/// Create an iterator over all but the last `n` elements of `iter`.
///
/// # Examples
///
/// ```rust,ignore
/// let data = [1, 2, 3, 4, 5];
/// let n = 2;
/// let mut iter = take_all_but(data.iter(), n);
/// assert_eq!(Some(4), iter.next());
/// assert_eq!(Some(5), iter.next());
/// assert_eq!(None, iter.next());
/// ```
pub fn take_all_but<I: Iterator>(iter: I, n: usize) -> TakeAllBut<I> {
    TakeAllBut::new(iter, n)
}

/// An iterator that only iterates over the last elements of another iterator.
pub struct TakeAllBut<I: Iterator> {
    iter: I,
    buf: RingBuffer<<I as Iterator>::Item>,
}

impl<I: Iterator> TakeAllBut<I> {
    pub fn new(mut iter: I, n: usize) -> Self {
        // Create a new ring buffer and fill it up.
        //
        // If there are fewer than `n` elements in `iter`, then we
        // exhaust the iterator so that whenever `TakeAllBut::next()` is
        // called, it will return `None`, as expected.
        let mut buf = RingBuffer::new(n);
        for _ in 0..n {
            let value = match iter.next() {
                None => {
                    break;
                }
                Some(x) => x,
            };
            buf.push_back(value);
        }
        Self { iter, buf }
    }
}

impl<I: Iterator> Iterator for TakeAllBut<I>
where
    I: Iterator,
{
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        match self.iter.next() {
            Some(value) => self.buf.push_back(value),
            None => None,
        }
    }
}

/// Like `std::io::Take`, but for lines instead of bytes.
///
/// This struct is generally created by calling [`take_lines`] on a
/// reader. Please see the documentation of [`take_lines`] for more
/// details.
pub struct TakeLines<T> {
    inner: T,
    limit: u64,
    separator: u8,
}

impl<T: Read> Read for TakeLines<T> {
    /// Read bytes from a buffer up to the requested number of lines.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        match self.inner.read(buf) {
            Ok(0) => Ok(0),
            Ok(n) => {
                for i in memchr_iter(self.separator, &buf[..n]) {
                    self.limit -= 1;
                    if self.limit == 0 {
                        return Ok(i + 1);
                    }
                }
                Ok(n)
            }
            Err(e) => Err(e),
        }
    }
}

/// Create an adaptor that will read at most `limit` lines from a given reader.
///
/// This function returns a new instance of `Read` that will read at
/// most `limit` lines, after which it will always return EOF
/// (`Ok(0)`).
///
/// The `separator` defines the character to interpret as the line
/// ending. For the usual notion of "line", set this to `b'\n'`.
pub fn take_lines<R>(reader: R, limit: u64, separator: u8) -> TakeLines<R> {
    TakeLines {
        inner: reader,
        limit,
        separator,
    }
}

#[cfg(test)]
mod tests {

    use std::io::BufRead;
    use std::io::BufReader;

    use crate::take::take_all_but;
    use crate::take::take_lines;

    #[test]
    fn test_fewer_elements() {
        let mut iter = take_all_but([0, 1, 2].iter(), 2);
        assert_eq!(Some(&0), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_same_number_of_elements() {
        let mut iter = take_all_but([0, 1].iter(), 2);
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_more_elements() {
        let mut iter = take_all_but([0].iter(), 2);
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_zero_elements() {
        let mut iter = take_all_but([0, 1, 2].iter(), 0);
        assert_eq!(Some(&0), iter.next());
        assert_eq!(Some(&1), iter.next());
        assert_eq!(Some(&2), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_zero_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 0, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_fewer_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 2, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(Some(String::from("a")), iter.next());
        assert_eq!(Some(String::from("b")), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_more_lines() {
        let input_reader = std::io::Cursor::new("a\nb\nc\n");
        let output_reader = BufReader::new(take_lines(input_reader, 4, b'\n'));
        let mut iter = output_reader.lines().map(|l| l.unwrap());
        assert_eq!(Some(String::from("a")), iter.next());
        assert_eq!(Some(String::from("b")), iter.next());
        assert_eq!(Some(String::from("c")), iter.next());
        assert_eq!(None, iter.next());
    }
}
