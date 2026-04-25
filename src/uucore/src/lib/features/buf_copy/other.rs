// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//!
//! Buffer-based copying implementation for other platforms.

use std::io::{Read, Write};

use crate::error::UResult;

/// Copy data from `Read` implementor `source` into a `Write` implementor
/// `dest`. This works by reading a chunk of data from `source` and writing the
/// data to `dest` in a loop, using std::io::copy. This is implemented for
/// non-Linux platforms.
///
/// # Arguments
/// * `source` - `Read` implementor to copy data from.
/// * `dest` - `Write` implementor to copy data to.
pub fn copy_stream<R, S>(src: &mut R, dest: &mut S) -> UResult<()>
where
    R: Read,
    S: Write,
{
    std::io::copy(src, dest)?;
    Ok(())
}
