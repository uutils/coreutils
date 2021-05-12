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
/// let buffer: RingBuffer<u8> = RingBuffer::new(2);
/// buffer.push_back(0);
/// buffer.push_back(1);
/// buffer.push_back(2);
/// assert_eq!(vec![1, 2], buffer.data);
/// ```
///
/// Take the last `n` elements from an iterator:
///
/// ```rust,ignore
/// let iter = vec![0, 1, 2, 3].iter();
/// assert_eq!(vec![2, 3], RingBuffer::from_iter(iter, 2).data);
/// ```
pub struct RingBuffer<T> {
    pub data: VecDeque<T>,
    size: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(size: usize) -> RingBuffer<T> {
        RingBuffer {
            data: VecDeque::new(),
            size,
        }
    }

    pub fn from_iter(iter: impl Iterator<Item = T>, size: usize) -> RingBuffer<T> {
        let mut ringbuf = RingBuffer::new(size);
        for value in iter {
            ringbuf.push_back(value);
        }
        ringbuf
    }

    pub fn push_back(&mut self, value: T) {
        if self.size <= self.data.len() {
            self.data.pop_front();
        }
        self.data.push_back(value)
    }
}
