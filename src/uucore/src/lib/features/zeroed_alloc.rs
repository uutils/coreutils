// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Fallible zero-initialized allocations.

/// Creates a zero-initialized boxed slice, returning an error if allocation fails.
///
/// `T` must implement [`zerocopy::FromZeros`], which guarantees that an all-zero
/// byte sequence is a valid value of that type.
pub fn try_new_boxed_slice<T: zerocopy::FromZeros>(len: usize) -> std::io::Result<Box<[T]>> {
    <[T] as zerocopy::FromZeros>::new_box_zeroed_with_elems(len)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

/// Creates a zero-initialized vector, returning an error if allocation fails.
///
/// `T` must implement [`zerocopy::FromZeros`], which guarantees that an all-zero
/// byte sequence is a valid value of that type.
pub fn try_new_vec<T: zerocopy::FromZeros>(len: usize) -> std::io::Result<Vec<T>> {
    <T as zerocopy::FromZeros>::new_vec_zeroed(len)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))
}

#[cfg(test)]
mod tests {
    use super::{try_new_boxed_slice, try_new_vec};

    #[test]
    fn test_try_new_vec() {
        assert_eq!(try_new_vec::<u16>(3).unwrap(), [0_u16; 3]);
        assert_eq!(
            try_new_vec::<u8>(usize::MAX).unwrap_err().kind(),
            std::io::ErrorKind::OutOfMemory
        );
    }

    #[test]
    fn test_try_new_boxed_slice() {
        assert_eq!(&*try_new_boxed_slice::<u16>(3).unwrap(), &[0_u16; 3]);
        assert_eq!(&*try_new_boxed_slice::<u8>(0).unwrap(), &[]);
        assert_eq!(
            try_new_boxed_slice::<u8>(usize::MAX).unwrap_err().kind(),
            std::io::ErrorKind::OutOfMemory
        );
    }
}
