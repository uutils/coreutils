// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::iter::repeat_n;

use crate::formatter_item_info::{FormatWriter, FormatterItemInfo};

// The writers below run once per item of input, so they write the digits
// themselves rather than going through the `fmt` machinery.

/// Appends a space and `p` in octal, zero-padded to `width` digits.
fn push_oct(out: &mut String, p: u64, width: usize) {
    push_power_of_two_radix(out, p, width, 3);
}

/// Appends a space and `p` in lowercase hexadecimal, zero-padded to `width`
/// digits.
fn push_hex(out: &mut String, p: u64, width: usize) {
    push_power_of_two_radix(out, p, width, 4);
}

/// Appends a space and `p` in base `2^bits`, zero-padded to `width` digits.
fn push_power_of_two_radix(out: &mut String, mut p: u64, width: usize, bits: u32) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    // Enough for a u64 in octal, the smallest radix used.
    let mut digits = [0; 22];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = DIGITS[(p & ((1 << bits) - 1)) as usize];
        p >>= bits;
        if p == 0 {
            break;
        }
    }
    let digits = &digits[start..];
    out.push(' ');
    out.extend(repeat_n('0', width.saturating_sub(digits.len())));
    out.extend(digits.iter().map(|&d| char::from(d)));
}

/// Appends a space and `magnitude` in decimal, with a minus sign if `negative`,
/// right-aligned with spaces to `width` characters.
fn push_dec(out: &mut String, mut magnitude: u64, negative: bool, width: usize) {
    let mut digits = [0; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }
    let digits = &digits[start..];
    out.push(' ');
    out.extend(repeat_n(
        ' ',
        width.saturating_sub(digits.len() + usize::from(negative)),
    ));
    if negative {
        out.push('-');
    }
    out.extend(digits.iter().map(|&d| char::from(d)));
}

/// Appends a space and `p` in decimal, right-aligned to `width` characters.
fn push_dec_unsigned(out: &mut String, p: u64, width: usize) {
    push_dec(out, p, false, width);
}

/// defines a static struct of type `FormatterItemInfo` called `$NAME`
///
/// Used to format unsigned integer types with help of a function called `$function`
/// `$byte_size` is the size of the type, `$print_width` is the maximum width in
/// human-readable format. `$push` is one of `push_oct`, `push_hex` or
/// `push_dec_unsigned`
macro_rules! int_writer_unsigned {
    ($NAME:ident, $byte_size:expr, $print_width:expr, $function:ident, $push:ident) => {
        fn $function(p: u64, out: &mut String) {
            $push(out, p, $print_width - 1);
        }

        pub static $NAME: FormatterItemInfo = FormatterItemInfo {
            byte_size: $byte_size,
            print_width: $print_width,
            formatter: FormatWriter::IntWriter($function),
        };
    };
}

/// defines a static struct of type `FormatterItemInfo` called `$NAME`
///
/// Used to format signed integer types in decimal with help of a function
/// called `$function`. `$byte_size` is the size of the type, `$print_width` is
/// the maximum width in human-readable format.
macro_rules! int_writer_signed {
    ($NAME:ident, $byte_size:expr, $print_width:expr, $function:ident) => {
        fn $function(p: u64, out: &mut String) {
            let s = sign_extend(p, $byte_size);
            push_dec(out, s.unsigned_abs(), s < 0, $print_width - 1);
        }

        pub static $NAME: FormatterItemInfo = FormatterItemInfo {
            byte_size: $byte_size,
            print_width: $print_width,
            formatter: FormatWriter::IntWriter($function),
        };
    };
}

/// Extends a signed number in `item` of `item_bytes` bytes into a (signed) i64
fn sign_extend(item: u64, item_bytes: usize) -> i64 {
    let shift = 64 - item_bytes * 8;
    (item << shift) as i64 >> shift
}

int_writer_unsigned!(FORMAT_ITEM_OCT8, 1, 4, format_item_oct8, push_oct); // max: 377
int_writer_unsigned!(FORMAT_ITEM_OCT16, 2, 7, format_item_oct16, push_oct); // max: 177777
int_writer_unsigned!(FORMAT_ITEM_OCT32, 4, 12, format_item_oct32, push_oct); // max: 37777777777
int_writer_unsigned!(FORMAT_ITEM_OCT64, 8, 23, format_item_oct64, push_oct); // max: 1777777777777777777777

int_writer_unsigned!(FORMAT_ITEM_HEX8, 1, 3, format_item_hex8, push_hex); // max: ff
int_writer_unsigned!(FORMAT_ITEM_HEX16, 2, 5, format_item_hex16, push_hex); // max: ffff
int_writer_unsigned!(FORMAT_ITEM_HEX32, 4, 9, format_item_hex32, push_hex); // max: ffffffff
int_writer_unsigned!(FORMAT_ITEM_HEX64, 8, 17, format_item_hex64, push_hex); // max: ffffffffffffffff

int_writer_unsigned!(
    FORMAT_ITEM_DEC8U,
    1,
    4,
    format_item_dec_u8,
    push_dec_unsigned
); // max: 255
int_writer_unsigned!(
    FORMAT_ITEM_DEC16U,
    2,
    6,
    format_item_dec_u16,
    push_dec_unsigned
); // max: 65535
int_writer_unsigned!(
    FORMAT_ITEM_DEC32U,
    4,
    11,
    format_item_dec_u32,
    push_dec_unsigned
); // max: 4294967295
int_writer_unsigned!(
    FORMAT_ITEM_DEC64U,
    8,
    21,
    format_item_dec_u64,
    push_dec_unsigned
); // max: 18446744073709551615

int_writer_signed!(FORMAT_ITEM_DEC8S, 1, 5, format_item_dec_s8); // max: -128
int_writer_signed!(FORMAT_ITEM_DEC16S, 2, 7, format_item_dec_s16); // max: -32768
int_writer_signed!(FORMAT_ITEM_DEC32S, 4, 12, format_item_dec_s32); // max: -2147483648
int_writer_signed!(FORMAT_ITEM_DEC64S, 8, 21, format_item_dec_s64); // max: -9223372036854775808

/// Runs an integer writer on its own, for the tests.
#[cfg(test)]
fn formatted(writer: fn(u64, &mut String), p: u64) -> String {
    let mut out = String::new();
    writer(p, &mut out);
    out
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_sign_extend() {
    assert_eq!(
        0xffff_ffff_ffff_ff80u64 as i64,
        sign_extend(0x0000_0000_0000_0080, 1)
    );
    assert_eq!(
        0xffff_ffff_ffff_8000u64 as i64,
        sign_extend(0x0000_0000_0000_8000, 2)
    );
    assert_eq!(
        0xffff_ffff_ff80_0000u64 as i64,
        sign_extend(0x0000_0000_0080_0000, 3)
    );
    assert_eq!(
        0xffff_ffff_8000_0000u64 as i64,
        sign_extend(0x0000_0000_8000_0000, 4)
    );
    assert_eq!(
        0xffff_ff80_0000_0000u64 as i64,
        sign_extend(0x0000_0080_0000_0000, 5)
    );
    assert_eq!(
        0xffff_8000_0000_0000u64 as i64,
        sign_extend(0x0000_8000_0000_0000, 6)
    );
    assert_eq!(
        0xff80_0000_0000_0000u64 as i64,
        sign_extend(0x0080_0000_0000_0000, 7)
    );
    assert_eq!(
        0x8000_0000_0000_0000u64 as i64,
        sign_extend(0x8000_0000_0000_0000, 8)
    );

    assert_eq!(0x0000_0000_0000_007f, sign_extend(0x0000_0000_0000_007f, 1));
    assert_eq!(0x0000_0000_0000_7fff, sign_extend(0x0000_0000_0000_7fff, 2));
    assert_eq!(0x0000_0000_007f_ffff, sign_extend(0x0000_0000_007f_ffff, 3));
    assert_eq!(0x0000_0000_7fff_ffff, sign_extend(0x0000_0000_7fff_ffff, 4));
    assert_eq!(0x0000_007f_ffff_ffff, sign_extend(0x0000_007f_ffff_ffff, 5));
    assert_eq!(0x0000_7fff_ffff_ffff, sign_extend(0x0000_7fff_ffff_ffff, 6));
    assert_eq!(0x007f_ffff_ffff_ffff, sign_extend(0x007f_ffff_ffff_ffff, 7));
    assert_eq!(0x7fff_ffff_ffff_ffff, sign_extend(0x7fff_ffff_ffff_ffff, 8));
}

#[test]
fn test_format_item_oct() {
    assert_eq!(" 000", formatted(format_item_oct8, 0));
    assert_eq!(" 377", formatted(format_item_oct8, 0xff));
    assert_eq!(" 000000", formatted(format_item_oct16, 0));
    assert_eq!(" 177777", formatted(format_item_oct16, 0xffff));
    assert_eq!(" 00000000000", formatted(format_item_oct32, 0));
    assert_eq!(" 37777777777", formatted(format_item_oct32, 0xffff_ffff));
    assert_eq!(" 0000000000000000000000", formatted(format_item_oct64, 0));
    assert_eq!(
        " 1777777777777777777777",
        formatted(format_item_oct64, 0xffff_ffff_ffff_ffff)
    );
}

#[test]
fn test_format_item_hex() {
    assert_eq!(" 00", formatted(format_item_hex8, 0));
    assert_eq!(" ff", formatted(format_item_hex8, 0xff));
    assert_eq!(" 0000", formatted(format_item_hex16, 0));
    assert_eq!(" ffff", formatted(format_item_hex16, 0xffff));
    assert_eq!(" 00000000", formatted(format_item_hex32, 0));
    assert_eq!(" ffffffff", formatted(format_item_hex32, 0xffff_ffff));
    assert_eq!(" 0000000000000000", formatted(format_item_hex64, 0));
    assert_eq!(
        " ffffffffffffffff",
        formatted(format_item_hex64, 0xffff_ffff_ffff_ffff)
    );
}

#[test]
fn test_format_item_dec_u() {
    assert_eq!("   0", formatted(format_item_dec_u8, 0));
    assert_eq!(" 255", formatted(format_item_dec_u8, 0xff));
    assert_eq!("     0", formatted(format_item_dec_u16, 0));
    assert_eq!(" 65535", formatted(format_item_dec_u16, 0xffff));
    assert_eq!("          0", formatted(format_item_dec_u32, 0));
    assert_eq!(" 4294967295", formatted(format_item_dec_u32, 0xffff_ffff));
    assert_eq!("                    0", formatted(format_item_dec_u64, 0));
    assert_eq!(
        " 18446744073709551615",
        formatted(format_item_dec_u64, 0xffff_ffff_ffff_ffff)
    );
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_format_item_dec_s() {
    assert_eq!("    0", formatted(format_item_dec_s8, 0));
    assert_eq!("  127", formatted(format_item_dec_s8, 0x7f));
    assert_eq!(" -128", formatted(format_item_dec_s8, 0x80));
    assert_eq!("      0", formatted(format_item_dec_s16, 0));
    assert_eq!("  32767", formatted(format_item_dec_s16, 0x7fff));
    assert_eq!(" -32768", formatted(format_item_dec_s16, 0x8000));
    assert_eq!("           0", formatted(format_item_dec_s32, 0));
    assert_eq!("  2147483647", formatted(format_item_dec_s32, 0x7fff_ffff));
    assert_eq!(" -2147483648", formatted(format_item_dec_s32, 0x8000_0000));
    assert_eq!("                    0", formatted(format_item_dec_s64, 0));
    assert_eq!(
        "  9223372036854775807",
        formatted(format_item_dec_s64, 0x7fff_ffff_ffff_ffff)
    );
    assert_eq!(
        " -9223372036854775808",
        formatted(format_item_dec_s64, 0x8000_0000_0000_0000)
    );
}

#[test]
fn test_writers_match_std_formatting() {
    type Writer = fn(u64, &mut String);
    let values = (0..=0xffff_u64).chain([
        0x7f,
        0x80,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x1_0000_0000,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0xffff_ffff_ffff_ffff,
    ]);
    for p in values {
        for (bytes, mask) in [(1, 0xff), (2, 0xffff), (4, 0xffff_ffff), (8, u64::MAX)] {
            let p = p & mask;
            let s = sign_extend(p, bytes);
            let (oct, hex, dec_u, dec_s): (Writer, Writer, Writer, Writer) = match bytes {
                1 => (
                    format_item_oct8,
                    format_item_hex8,
                    format_item_dec_u8,
                    format_item_dec_s8,
                ),
                2 => (
                    format_item_oct16,
                    format_item_hex16,
                    format_item_dec_u16,
                    format_item_dec_s16,
                ),
                4 => (
                    format_item_oct32,
                    format_item_hex32,
                    format_item_dec_u32,
                    format_item_dec_s32,
                ),
                _ => (
                    format_item_oct64,
                    format_item_hex64,
                    format_item_dec_u64,
                    format_item_dec_s64,
                ),
            };
            let (oct_w, hex_w, dec_u_w, dec_s_w) = match bytes {
                1 => (3, 2, 3, 4),
                2 => (6, 4, 5, 6),
                4 => (11, 8, 10, 11),
                _ => (22, 16, 20, 20),
            };
            assert_eq!(formatted(oct, p), format!(" {p:0oct_w$o}"));
            assert_eq!(formatted(hex, p), format!(" {p:0hex_w$x}"));
            assert_eq!(formatted(dec_u, p), format!(" {p:dec_u_w$}"));
            assert_eq!(formatted(dec_s, p), format!(" {s:dec_s_w$}"));
        }
    }
}
