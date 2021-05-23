//! Take all but the last elements of an iterator.
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
    pub fn new(mut iter: I, n: usize) -> TakeAllBut<I> {
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
        TakeAllBut { iter, buf }
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

#[cfg(test)]
mod tests {

    use crate::take::take_all_but;

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
}
