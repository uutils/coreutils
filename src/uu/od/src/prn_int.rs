use crate::formatteriteminfo::*;

/// format string to print octal using `int_writer_unsigned`
macro_rules! OCT {
    () => {
        " {:0width$o}"
    };
}
/// format string to print hexadecimal using `int_writer_unsigned`
macro_rules! HEX {
    () => {
        " {:0width$x}"
    };
}
/// format string to print decimal using `int_writer_unsigned` or `int_writer_signed`
macro_rules! DEC {
    () => {
        " {:width$}"
    };
}

/// defines a static struct of type `FormatterItemInfo` called `$NAME`
///
/// Used to format unsigned integer types with help of a function called `$function`
/// `$byte_size` is the size of the type, `$print_width` is the maximum width in
/// human-readable format. `$format_str` is one of OCT, HEX or DEC
macro_rules! int_writer_unsigned {
    ($NAME:ident, $byte_size:expr, $print_width:expr, $function:ident, $format_str:expr) => {
        fn $function(p: u64) -> String {
            format!($format_str, p, width = $print_width - 1)
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
/// Used to format signed integer types with help of a function called `$function`
/// `$byte_size` is the size of the type, `$print_width` is the maximum width in
/// human-readable format. `$format_str` should be DEC
macro_rules! int_writer_signed {
    ($NAME:ident, $byte_size:expr, $print_width:expr, $function:ident, $format_str:expr) => {
        fn $function(p: u64) -> String {
            let s = sign_extend(p, $byte_size);
            format!($format_str, s, width = $print_width - 1)
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

int_writer_unsigned!(FORMAT_ITEM_OCT8, 1, 4, format_item_oct8, OCT!()); // max: 377
int_writer_unsigned!(FORMAT_ITEM_OCT16, 2, 7, format_item_oct16, OCT!()); // max: 177777
int_writer_unsigned!(FORMAT_ITEM_OCT32, 4, 12, format_item_oct32, OCT!()); // max: 37777777777
int_writer_unsigned!(FORMAT_ITEM_OCT64, 8, 23, format_item_oct64, OCT!()); // max: 1777777777777777777777

int_writer_unsigned!(FORMAT_ITEM_HEX8, 1, 3, format_item_hex8, HEX!()); // max: ff
int_writer_unsigned!(FORMAT_ITEM_HEX16, 2, 5, format_item_hex16, HEX!()); // max: ffff
int_writer_unsigned!(FORMAT_ITEM_HEX32, 4, 9, format_item_hex32, HEX!()); // max: ffffffff
int_writer_unsigned!(FORMAT_ITEM_HEX64, 8, 17, format_item_hex64, HEX!()); // max: ffffffffffffffff

int_writer_unsigned!(FORMAT_ITEM_DEC8U, 1, 4, format_item_dec_u8, DEC!()); // max: 255
int_writer_unsigned!(FORMAT_ITEM_DEC16U, 2, 6, format_item_dec_u16, DEC!()); // max: 65535
int_writer_unsigned!(FORMAT_ITEM_DEC32U, 4, 11, format_item_dec_u32, DEC!()); // max: 4294967295
int_writer_unsigned!(FORMAT_ITEM_DEC64U, 8, 21, format_item_dec_u64, DEC!()); // max: 18446744073709551615

int_writer_signed!(FORMAT_ITEM_DEC8S, 1, 5, format_item_dec_s8, DEC!()); // max: -128
int_writer_signed!(FORMAT_ITEM_DEC16S, 2, 7, format_item_dec_s16, DEC!()); // max: -32768
int_writer_signed!(FORMAT_ITEM_DEC32S, 4, 12, format_item_dec_s32, DEC!()); // max: -2147483648
int_writer_signed!(FORMAT_ITEM_DEC64S, 8, 21, format_item_dec_s64, DEC!()); // max: -9223372036854775808

#[test]
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
    assert_eq!(" 000", format_item_oct8(0));
    assert_eq!(" 377", format_item_oct8(0xff));
    assert_eq!(" 000000", format_item_oct16(0));
    assert_eq!(" 177777", format_item_oct16(0xffff));
    assert_eq!(" 00000000000", format_item_oct32(0));
    assert_eq!(" 37777777777", format_item_oct32(0xffff_ffff));
    assert_eq!(" 0000000000000000000000", format_item_oct64(0));
    assert_eq!(
        " 1777777777777777777777",
        format_item_oct64(0xffff_ffff_ffff_ffff)
    );
}

#[test]
fn test_format_item_hex() {
    assert_eq!(" 00", format_item_hex8(0));
    assert_eq!(" ff", format_item_hex8(0xff));
    assert_eq!(" 0000", format_item_hex16(0));
    assert_eq!(" ffff", format_item_hex16(0xffff));
    assert_eq!(" 00000000", format_item_hex32(0));
    assert_eq!(" ffffffff", format_item_hex32(0xffff_ffff));
    assert_eq!(" 0000000000000000", format_item_hex64(0));
    assert_eq!(
        " ffffffffffffffff",
        format_item_hex64(0xffff_ffff_ffff_ffff)
    );
}

#[test]
fn test_format_item_dec_u() {
    assert_eq!("   0", format_item_dec_u8(0));
    assert_eq!(" 255", format_item_dec_u8(0xff));
    assert_eq!("     0", format_item_dec_u16(0));
    assert_eq!(" 65535", format_item_dec_u16(0xffff));
    assert_eq!("          0", format_item_dec_u32(0));
    assert_eq!(" 4294967295", format_item_dec_u32(0xffff_ffff));
    assert_eq!("                    0", format_item_dec_u64(0));
    assert_eq!(
        " 18446744073709551615",
        format_item_dec_u64(0xffff_ffff_ffff_ffff)
    );
}

#[test]
fn test_format_item_dec_s() {
    assert_eq!("    0", format_item_dec_s8(0));
    assert_eq!("  127", format_item_dec_s8(0x7f));
    assert_eq!(" -128", format_item_dec_s8(0x80));
    assert_eq!("      0", format_item_dec_s16(0));
    assert_eq!("  32767", format_item_dec_s16(0x7fff));
    assert_eq!(" -32768", format_item_dec_s16(0x8000));
    assert_eq!("           0", format_item_dec_s32(0));
    assert_eq!("  2147483647", format_item_dec_s32(0x7fff_ffff));
    assert_eq!(" -2147483648", format_item_dec_s32(0x8000_0000));
    assert_eq!("                    0", format_item_dec_s64(0));
    assert_eq!(
        "  9223372036854775807",
        format_item_dec_s64(0x7fff_ffff_ffff_ffff)
    );
    assert_eq!(
        " -9223372036854775808",
        format_item_dec_s64(0x8000_0000_0000_0000)
    );
}
