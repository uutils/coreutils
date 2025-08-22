// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bfloat bigdecimal extendedbigdecimal multifile

use half::{bf16, f16};
use std::io;
use uucore::extendedbigdecimal::ExtendedBigDecimal;

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

    /// Returns an `ExtendedBigDecimal` from the internal buffer at position `start`.
    /// Only able to parse 16-bytes padded "f80", at least for now
    pub fn read_extended_big_decimal(&self, start: usize, byte_size: usize) -> ExtendedBigDecimal {
        assert!(byte_size == 16, "Invalid byte_size: {byte_size}");
        let data = self.byte_order.read_u128(&self.data[start..start + 16]);

        fn bits(data: u128, offset: usize, size: usize) -> u64 {
            (data >> offset & (u128::MAX >> (128 - size))) as u64
        }

        // Parse an f80 number, see https://en.wikipedia.org/wiki/Extended_precision for details.
        // let _pad = bits(data, 80, 48); // Top 48 bits of padding ignored.
        let sign = bits(data, 79, 1);
        let exp = bits(data, 64, 15) as i64;
        let one = bits(data, 63, 1);
        // m includes the leading `one`, and needs to be divided by 2**63
        let m = bits(data, 0, 64);

        let ebd = if exp == 0 {
            // Can be zero, subnormal or pseudo-subnormal, but the computation is always the same.
            ExtendedBigDecimal::from_number_exp2(m, -16382 - 63)
        } else if exp == 0x7fff {
            if one == 0 {
                // Pseudo-infinity or Pseudo-NaN, both treated as nan.
                ExtendedBigDecimal::Nan
            } else if m == (1u64 << 63) {
                // one == 1, frac == 0
                ExtendedBigDecimal::Infinity
            } else {
                // one == 1, frac != 0
                ExtendedBigDecimal::Nan
            }
        } else {
            // exp is not all 0 or 1.
            if one == 0 {
                // Un-normal, treat as nan
                ExtendedBigDecimal::Nan
            } else {
                ExtendedBigDecimal::from_number_exp2(m, exp - 16383 - 63)
            }
        };

        if sign == 1 { -ebd } else { ebd }
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
    #[allow(clippy::cognitive_complexity)]
    fn smoke_test() {
        let data = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xff, 0xbf, 0xca, 0xfe, 0xca, 0xfe,
            0xca, 0xfe, 0xff, 0xff,
        ];
        let mut input = PeekReader::new(Cursor::new(&data));
        let mut sut = InputDecoder::new(&mut input, 16, 2, ByteOrder::Little);

        // Peek normal length
        let mut mem = sut.peek_read().unwrap();

        assert_eq!(16, mem.length());

        assert_eq!(-2.0, mem.read_float(0, 8));
        assert_eq!(-2.0, mem.read_float(4, 4));
        // sign = 1 (negative)
        // exp = 0x3fff
        assert_eq!(
            Into::<ExtendedBigDecimal>::into(-1.5),
            mem.read_extended_big_decimal(0, 16)
        );
        assert_eq!(0xc000_0000_0000_0000, mem.read_uint(0, 8));
        assert_eq!(0xc000_0000, mem.read_uint(4, 4));
        assert_eq!(0xc000, mem.read_uint(6, 2));
        assert_eq!(0xc0, mem.read_uint(7, 1));
        assert_eq!(&[0xca, 0xfe], mem.get_buffer(14));
        assert_eq!(&[0xca, 0xfe, 0xff, 0xff], mem.get_full_buffer(14));

        let mut copy: Vec<u8> = Vec::new();
        mem.clone_buffer(&mut copy);
        assert_eq!(
            vec![
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xff, 0xbf, 0xca, 0xfe, 0xca, 0xfe,
                0xca, 0xfe
            ],
            copy
        );

        mem.zero_out_buffer(14, 16);
        assert_eq!(&[0, 0, 0xff, 0xff], mem.get_full_buffer(14));

        // Peek tail
        let mem = sut.peek_read().unwrap();
        assert_eq!(2, mem.length());
        assert_eq!(0xffff, mem.read_uint(0, 2));
    }
}
