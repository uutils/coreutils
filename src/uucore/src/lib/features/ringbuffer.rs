//! A fixed-size ring buffer.
use std::collections::VecDeque;

/// A fixed-size ring buffer backed by a `VecDeque`.
///
/// If the ring buffer is not full, then calling the [`push_back`]
/// method appends elements, as in a [`VecDeque`]. If the ring buffer
/// is full, then calling [`push_back`] removes the element at the
/// front of the buffer (in a first-in, first-out manner) before
/// appending the new element to the back of the buffer.
///
/// Use [`from_iter`] to take the last `size` elements from an
/// iterator.
///
/// # Examples
///
/// After exceeding the size limit, the oldest elements are dropped in
/// favor of the newest element:
///
/// ```rust,ignore
/// let mut buffer: RingBuffer<u8> = RingBuffer::new(2);
/// buffer.push_back(0);
/// buffer.push_back(1);
/// buffer.push_back(2);
/// assert_eq!(vec![1, 2], buffer.data);
/// ```
///
/// Take the last `n` elements from an iterator:
///
/// ```rust,ignore
/// let iter = [0, 1, 2].iter();
/// let actual = RingBuffer::from_iter(iter, 2).data;
/// let expected = VecDeque::from_iter([1, 2].iter());
/// assert_eq!(expected, actual);
/// ```
///
/// [`push_back`]: struct.RingBuffer.html#method.push_back
/// [`from_iter`]: struct.RingBuffer.html#method.from_iter
pub struct RingBuffer<T> {
    pub data: VecDeque<T>,
    size: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: VecDeque::new(),
            size,
        }
    }

    pub fn from_iter(iter: impl Iterator<Item = T>, size: usize) -> Self {
        let mut ring_buffer = Self::new(size);
        for value in iter {
            ring_buffer.push_back(value);
        }
        ring_buffer
    }

    /// Append a value to the end of the ring buffer.
    ///
    /// If the ring buffer is not full, this method return [`None`]. If
    /// the ring buffer is full, appending a new element will cause the
    /// oldest element to be evicted. In that case this method returns
    /// that element, or `None`.
    ///
    /// In the special case where the size limit is zero, each call to
    /// this method with input `value` returns `Some(value)`, because
    /// the input is immediately evicted.
    ///
    /// # Examples
    ///
    /// Appending an element when the buffer is full returns the oldest
    /// element:
    ///
    /// ```rust,ignore
    /// let mut buf = RingBuffer::new(3);
    /// assert_eq!(None, buf.push_back(0));
    /// assert_eq!(None, buf.push_back(1));
    /// assert_eq!(None, buf.push_back(2));
    /// assert_eq!(Some(0), buf.push_back(3));
    /// ```
    ///
    /// If the size limit is zero, then this method always returns the
    /// input value:
    ///
    /// ```rust,ignore
    /// let mut buf = RingBuffer::new(0);
    /// assert_eq!(Some(0), buf.push_back(0));
    /// assert_eq!(Some(1), buf.push_back(1));
    /// assert_eq!(Some(2), buf.push_back(2));
    /// ```
    pub fn push_back(&mut self, value: T) -> Option<T> {
        if self.size == 0 {
            return Some(value);
        }
        let result = if self.size <= self.data.len() {
            self.data.pop_front()
        } else {
            None
        };
        self.data.push_back(value);
        result
    }
}

#[cfg(test)]
mod tests {

    use crate::ringbuffer::RingBuffer;
    use std::collections::VecDeque;

    #[test]
    fn test_size_limit_zero() {
        let mut buf = RingBuffer::new(0);
        assert_eq!(Some(0), buf.push_back(0));
        assert_eq!(Some(1), buf.push_back(1));
        assert_eq!(Some(2), buf.push_back(2));
    }

    #[test]
    fn test_evict_oldest() {
        let mut buf = RingBuffer::new(2);
        assert_eq!(None, buf.push_back(0));
        assert_eq!(None, buf.push_back(1));
        assert_eq!(Some(0), buf.push_back(2));
    }

    #[test]
    fn test_from_iter() {
        let iter = [0, 1, 2].iter();
        let actual = RingBuffer::from_iter(iter, 2).data;
        let expected: VecDeque<&i32> = [1, 2].iter().collect();
        assert_eq!(expected, actual);
    }
}
