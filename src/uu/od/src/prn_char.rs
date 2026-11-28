// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::iter::repeat_n;

use crate::formatter_item_info::{FormatWriter, FormatterItemInfo};
use crate::prn_int::push_oct;

pub static FORMAT_ITEM_A: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 4,
    formatter: FormatWriter::IntWriter(format_item_a),
};

pub static FORMAT_ITEM_C: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 4,
    formatter: FormatWriter::MultibyteWriter(format_item_c),
};

static A_CHARS: [&str; 128] = [
    "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "ht", "nl", "vt", "ff", "cr",
    "so", "si", "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb", "can", "em", "sub", "esc",
    "fs", "gs", "rs", "us", "sp", "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-",
    ".", "/", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<", "=", ">", "?", "@",
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "^", "_", "`", "a", "b", "c", "d", "e", "f",
    "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y",
    "z", "{", "|", "}", "~", "del",
];

/// Appends `s` right-aligned to the 4 columns of a character item.
fn push_right_aligned(out: &mut String, s: &str) {
    out.extend(repeat_n(' ', 4usize.saturating_sub(s.len())));
    out.push_str(s);
}

fn format_item_a(p: u64, out: &mut String) {
    // item-bytes == 1
    let b = (p & 0x7f) as u8;
    push_right_aligned(out, A_CHARS.get(b as usize).unwrap_or(&"??"));
}

static C_CHARS: [&str; 128] = [
    "\\0", "001", "002", "003", "004", "005", "006", "\\a", "\\b", "\\t", "\\n", "\\v", "\\f",
    "\\r", "016", "017", "020", "021", "022", "023", "024", "025", "026", "027", "030", "031",
    "032", "033", "034", "035", "036", "037", " ", "!", "\"", "#", "$", "%", "&", "'", "(", ")",
    "*", "+", ",", "-", ".", "/", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<",
    "=", ">", "?", "@", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O",
    "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "^", "_", "`", "a", "b",
    "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u",
    "v", "w", "x", "y", "z", "{", "|", "}", "~", "177",
];

fn format_item_c(bytes: &[u8], out: &mut String) {
    // item-bytes == 1
    let b = bytes[0];

    if b & 0x80 == 0x00 {
        // ASCII byte (0xxxxxxx)
        push_right_aligned(out, C_CHARS[b as usize]);
    } else {
        // Continuation or leading byte of a multibyte UTF-8 sequence — treat as raw byte
        push_oct(out, b.into(), 3);
    }
}

pub fn format_ascii_dump(bytes: &[u8]) -> String {
    let mut result = String::new();

    result.push('>');
    for c in bytes {
        if *c >= 0x20 && *c <= 0x7e {
            result.push_str(C_CHARS[*c as usize]);
        } else {
            result.push('.');
        }
    }
    result.push('<');

    result
}

/// Runs `format_item_a` on its own, for the tests.
#[cfg(test)]
fn formatted_a(p: u64) -> String {
    let mut out = String::new();
    format_item_a(p, &mut out);
    out
}

/// Runs `format_item_c` on its own, for the tests.
#[cfg(test)]
fn formatted_c(bytes: &[u8]) -> String {
    let mut out = String::new();
    format_item_c(bytes, &mut out);
    out
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_format_item_a() {
    assert_eq!(" nul", formatted_a(0x00));
    assert_eq!(" soh", formatted_a(0x01));
    assert_eq!("  sp", formatted_a(0x20));
    assert_eq!("   A", formatted_a(0x41));
    assert_eq!("   ~", formatted_a(0x7e));
    assert_eq!(" del", formatted_a(0x7f));

    assert_eq!(" nul", formatted_a(0x80));
    assert_eq!("   A", formatted_a(0xc1));
    assert_eq!("   ~", formatted_a(0xfe));
    assert_eq!(" del", formatted_a(0xff));
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_format_item_c() {
    assert_eq!("  \\0", formatted_c(&[0x00]));
    assert_eq!(" 001", formatted_c(&[0x01]));
    assert_eq!("    ", formatted_c(&[0x20]));
    assert_eq!("   A", formatted_c(&[0x41]));
    assert_eq!("   ~", formatted_c(&[0x7e]));
    assert_eq!(" 177", formatted_c(&[0x7f]));
    assert_eq!("   A", formatted_c(&[0x41, 0x21]));

    assert_eq!(" 200", formatted_c(&[0x80]));
    assert_eq!(" 237", formatted_c(&[0x9f]));

    assert_eq!(" 303", formatted_c(&[0xc3, 0x9f]));
    assert_eq!(" 303", formatted_c(&[0xc3, 0x9f, 0x21]));

    assert_eq!(" 341", formatted_c(&[0xe1, 0x80, 0x80]));

    assert_eq!(" 360", formatted_c(&[0xf0, 0x9f, 0x92, 0x96]));

    assert_eq!(" 300", formatted_c(&[0xc0, 0x80])); // invalid utf-8 (UTF-8 null)
    assert_eq!(" 301", formatted_c(&[0xc1, 0xa1])); // invalid utf-8
    assert_eq!(" 303", formatted_c(&[0xc3, 0xc3])); // invalid utf-8
    assert_eq!(" 360", formatted_c(&[0xf0, 0x82, 0x82, 0xac])); // invalid utf-8 (overlong)
    assert_eq!(" 360", formatted_c(&[0xf0, 0x9f, 0x92])); // invalid utf-8 (missing octet)
    assert_eq!(" 364", formatted_c(&[0xf4, 0x8f, 0xbf, 0xbd])); // largest valid utf-8   // spell-checker:ignore 10FFFD FFFD
    assert_eq!(" 364", formatted_c(&[0xf4, 0x90, 0x00, 0x00])); // invalid utf-8
    assert_eq!(" 365", formatted_c(&[0xf5, 0x80, 0x80, 0x80])); // invalid utf-8
    assert_eq!(" 377", formatted_c(&[0xff])); // invalid utf-8
}

#[test]
fn test_format_ascii_dump() {
    assert_eq!(">.<", format_ascii_dump(&[0x00]));
    assert_eq!(
        ">. A~.<",
        format_ascii_dump(&[0x1f, 0x20, 0x41, 0x7e, 0x7f])
    );
}
