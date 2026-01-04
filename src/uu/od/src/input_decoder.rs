// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bfloat multifile mant

use half::{bf16, f16};
use std::io;

use crate::byteorder_io::ByteOrder;
use crate::multifile_reader::HasError;
use crate::peek_reader::PeekRead;

/// Processes an input and provides access to the data read in various formats
///
/// Currently only useful if the input implements `PeekRead`.
pub struct InputDecoder<'a, I>
where
    I: 'a,
{
    /// The input from which data is read
    input: &'a mut I,

    /// A memory buffer, it's size is set in `new`.
    data: Vec<u8>,
    /// The number of bytes in the buffer reserved for the peek data from `PeekRead`.
    reserved_peek_length: usize,

    /// The number of (valid) bytes in the buffer.
    used_normal_length: usize,
    /// The number of peek bytes in the buffer.
    used_peek_length: usize,

    /// Byte order used to read data from the buffer.
    byte_order: ByteOrder,
}

impl<I> InputDecoder<'_, I> {
    /// Creates a new `InputDecoder` with an allocated buffer of `normal_length` + `peek_length` bytes.
    /// `byte_order` determines how to read multibyte formats from the buffer.
    pub fn new(
        input: &mut I,
        normal_length: usize,
        peek_length: usize,
        byte_order: ByteOrder,
    ) -> InputDecoder<'_, I> {
        let bytes = vec![0; normal_length + peek_length];

        InputDecoder {
            input,
            data: bytes,
            reserved_peek_length: peek_length,
            used_normal_length: 0,
            used_peek_length: 0,
            byte_order,
        }
    }
}

impl<I> InputDecoder<'_, I>
where
    I: PeekRead,
{
    /// calls `peek_read` on the internal stream to (re)fill the buffer. Returns a
    /// `MemoryDecoder` providing access to the result or returns an i/o error.
    pub fn peek_read(&mut self) -> io::Result<MemoryDecoder<'_>> {
        match self
            .input
            .peek_read(self.data.as_mut_slice(), self.reserved_peek_length)
        {
            Ok((n, p)) => {
                self.used_normal_length = n;
                self.used_peek_length = p;
                Ok(MemoryDecoder {
                    data: &mut self.data,
                    used_normal_length: self.used_normal_length,
                    used_peek_length: self.used_peek_length,
                    byte_order: self.byte_order,
                })
            }
            Err(e) => Err(e),
        }
    }
}

impl<I> HasError for InputDecoder<'_, I>
where
    I: HasError,
{
    /// calls `has_error` on the internal stream.
    fn has_error(&self) -> bool {
        self.input.has_error()
    }
}

/// Provides access to the internal data in various formats
pub struct MemoryDecoder<'a> {
    /// A reference to the parents' data
    data: &'a mut Vec<u8>,
    /// The number of (valid) bytes in the buffer.
    used_normal_length: usize,
    /// The number of peek bytes in the buffer.
    used_peek_length: usize,
    /// Byte order used to read data from the buffer.
    byte_order: ByteOrder,
}

impl MemoryDecoder<'_> {
    /// Set a part of the internal buffer to zero.
    /// access to the whole buffer is possible, not just to the valid data.
    pub fn zero_out_buffer(&mut self, start: usize, end: usize) {
        for i in start..end {
            self.data[i] = 0;
        }
    }

    /// Returns the current length of the buffer. (ie. how much valid data it contains.)
    pub fn length(&self) -> usize {
        self.used_normal_length
    }

    /// Creates a clone of the internal buffer. The clone only contain the valid data.
    pub fn clone_buffer(&self, other: &mut Vec<u8>) {
        other.clone_from(self.data);
        other.resize(self.used_normal_length, 0);
    }

    /// Returns a slice to the internal buffer starting at `start`.
    pub fn get_buffer(&self, start: usize) -> &[u8] {
        &self.data[start..self.used_normal_length]
    }

    /// Returns a slice to the internal buffer including the peek data starting at `start`.
    pub fn get_full_buffer(&self, start: usize) -> &[u8] {
        &self.data[start..self.used_normal_length + self.used_peek_length]
    }

    /// Returns a u8/u16/u32/u64 from the internal buffer at position `start`.
    pub fn read_uint(&self, start: usize, byte_size: usize) -> u64 {
        match byte_size {
            1 => u64::from(self.data[start]),
            2 => u64::from(self.byte_order.read_u16(&self.data[start..start + 2])),
            4 => u64::from(self.byte_order.read_u32(&self.data[start..start + 4])),
            8 => self.byte_order.read_u64(&self.data[start..start + 8]),
            _ => panic!("Invalid byte_size: {byte_size}"),
        }
    }

    /// Returns a f32/f64 from the internal buffer at position `start`.
    pub fn read_float(&self, start: usize, byte_size: usize) -> f64 {
        match byte_size {
            2 => f64::from(f16::from_bits(
                self.byte_order.read_u16(&self.data[start..start + 2]),
            )),
            4 => f64::from(self.byte_order.read_f32(&self.data[start..start + 4])),
            8 => self.byte_order.read_f64(&self.data[start..start + 8]),
            _ => panic!("Invalid byte_size: {byte_size}"),
        }
    }

    /// Returns a bfloat16 as f64 from the internal buffer at position `start`.
    pub fn read_bfloat(&self, start: usize) -> f64 {
        let bits = self.byte_order.read_u16(&self.data[start..start + 2]);
        let val = f32::from(bf16::from_bits(bits));
        f64::from(val)
    }

    /// Returns a long double from the internal buffer at position `start`.
    /// We read 16 bytes as u128 (respecting endianness) and convert to f64.
    /// This ensures that endianness swapping works correctly even if we lose precision.
    pub fn read_long_double(&self, start: usize) -> f64 {
        let bits = self.byte_order.read_u128(&self.data[start..start + 16]);
        u128_to_f64(bits)
    }
}

fn u128_to_f64(u: u128) -> f64 {
    let sign = (u >> 127) as u64;
    let exp = ((u >> 112) & 0x7FFF) as u64;
    let mant = u & ((1 << 112) - 1);

    if exp == 0x7FFF {
        // Infinity or NaN
        if mant == 0 {
            if sign == 0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        } else {
            f64::NAN
        }
    } else if exp == 0 {
        // Subnormal or zero
        if mant == 0 {
            if sign == 0 { 0.0 } else { -0.0 }
        } else {
            // Subnormal f128 is too small for f64, flush to zero
            if sign == 0 { 0.0 } else { -0.0 }
        }
    } else {
        // Normal
        let new_exp = exp as i64 - 16383 + 1023;
        if new_exp >= 2047 {
            // Overflow to infinity
            if sign == 0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        } else if new_exp <= 0 {
            // Underflow to zero
            if sign == 0 { 0.0 } else { -0.0 }
        } else {
            // Normal f64
            // Mantissa: take top 52 bits of 112-bit mantissa
            let new_mant = (mant >> (112 - 52)) as u64;
            let bits = (sign << 63) | ((new_exp as u64) << 52) | new_mant;
            f64::from_bits(bits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byteorder_io::ByteOrder;
    use crate::peek_reader::PeekReader;
    use std::io::Cursor;

    #[test]
    #[allow(clippy::float_cmp)]
    fn smoke_test() {
        let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0xff, 0xff];
        let mut input = PeekReader::new(Cursor::new(&data));
        let mut sut = InputDecoder::new(&mut input, 8, 2, ByteOrder::Little);

        // Peek normal length
        let mut mem = sut.peek_read().unwrap();

        assert_eq!(8, mem.length());

        assert_eq!(-2.0, mem.read_float(0, 8));
        assert_eq!(-2.0, mem.read_float(4, 4));
        assert_eq!(0xc000_0000_0000_0000, mem.read_uint(0, 8));
        assert_eq!(0xc000_0000, mem.read_uint(4, 4));
        assert_eq!(0xc000, mem.read_uint(6, 2));
        assert_eq!(0xc0, mem.read_uint(7, 1));
        assert_eq!(&[0, 0xc0], mem.get_buffer(6));
        assert_eq!(&[0, 0xc0, 0xff, 0xff], mem.get_full_buffer(6));

        let mut copy: Vec<u8> = Vec::new();
        mem.clone_buffer(&mut copy);
        assert_eq!(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0], copy);

        mem.zero_out_buffer(7, 8);
        assert_eq!(&[0, 0, 0xff, 0xff], mem.get_full_buffer(6));

        // Peek tail
        let mem = sut.peek_read().unwrap();
        assert_eq!(2, mem.length());
        assert_eq!(0xffff, mem.read_uint(0, 2));
    }
}
