use std::str::from_utf8;

use crate::formatteriteminfo::*;

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

fn format_item_a(p: u64) -> String {
    // item-bytes == 1
    let b = (p & 0x7f) as u8;
    format!("{:>4}", A_CHARS.get(b as usize).unwrap_or(&"??"))
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

fn format_item_c(bytes: &[u8]) -> String {
    // item-bytes == 1
    let b = bytes[0];

    if b & 0x80 == 0x00 {
        match C_CHARS.get(b as usize) {
            Some(s) => format!("{:>4}", s),
            None => format!("{:>4}", b),
        }
    } else if (b & 0xc0) == 0x80 {
        // second or subsequent octet of an utf-8 sequence
        String::from("  **")
    } else if ((b & 0xe0) == 0xc0) && (bytes.len() >= 2) {
        // start of a 2 octet utf-8 sequence
        match from_utf8(&bytes[0..2]) {
            Ok(s) => format!("{:>4}", s),
            Err(_) => format!(" {:03o}", b),
        }
    } else if ((b & 0xf0) == 0xe0) && (bytes.len() >= 3) {
        // start of a 3 octet utf-8 sequence
        match from_utf8(&bytes[0..3]) {
            Ok(s) => format!("{:>4}", s),
            Err(_) => format!(" {:03o}", b),
        }
    } else if ((b & 0xf8) == 0xf0) && (bytes.len() >= 4) {
        // start of a 4 octet utf-8 sequence
        match from_utf8(&bytes[0..4]) {
            Ok(s) => format!("{:>4}", s),
            Err(_) => format!(" {:03o}", b),
        }
    } else {
        // invalid utf-8
        format!(" {:03o}", b)
    }
}

pub fn format_ascii_dump(bytes: &[u8]) -> String {
    let mut result = String::new();

    result.push('>');
    for c in bytes.iter() {
        if *c >= 0x20 && *c <= 0x7e {
            result.push_str(C_CHARS[*c as usize]);
        } else {
            result.push('.');
        }
    }
    result.push('<');

    result
}

#[test]
fn test_format_item_a() {
    assert_eq!(" nul", format_item_a(0x00));
    assert_eq!(" soh", format_item_a(0x01));
    assert_eq!("  sp", format_item_a(0x20));
    assert_eq!("   A", format_item_a(0x41));
    assert_eq!("   ~", format_item_a(0x7e));
    assert_eq!(" del", format_item_a(0x7f));

    assert_eq!(" nul", format_item_a(0x80));
    assert_eq!("   A", format_item_a(0xc1));
    assert_eq!("   ~", format_item_a(0xfe));
    assert_eq!(" del", format_item_a(0xff));
}

#[test]
fn test_format_item_c() {
    assert_eq!("  \\0", format_item_c(&[0x00]));
    assert_eq!(" 001", format_item_c(&[0x01]));
    assert_eq!("    ", format_item_c(&[0x20]));
    assert_eq!("   A", format_item_c(&[0x41]));
    assert_eq!("   ~", format_item_c(&[0x7e]));
    assert_eq!(" 177", format_item_c(&[0x7f]));
    assert_eq!("   A", format_item_c(&[0x41, 0x21]));

    assert_eq!("  **", format_item_c(&[0x80]));
    assert_eq!("  **", format_item_c(&[0x9f]));

    assert_eq!("   ß", format_item_c(&[0xc3, 0x9f]));
    assert_eq!("   ß", format_item_c(&[0xc3, 0x9f, 0x21]));

    assert_eq!("   \u{1000}", format_item_c(&[0xe1, 0x80, 0x80]));
    assert_eq!("   \u{1000}", format_item_c(&[0xe1, 0x80, 0x80, 0x21]));

    assert_eq!("   \u{1f496}", format_item_c(&[0xf0, 0x9f, 0x92, 0x96]));
    assert_eq!(
        "   \u{1f496}",
        format_item_c(&[0xf0, 0x9f, 0x92, 0x96, 0x21])
    );

    assert_eq!(" 300", format_item_c(&[0xc0, 0x80])); // invalid utf-8 (UTF-8 null)
    assert_eq!(" 301", format_item_c(&[0xc1, 0xa1])); // invalid utf-8
    assert_eq!(" 303", format_item_c(&[0xc3, 0xc3])); // invalid utf-8
    assert_eq!(" 360", format_item_c(&[0xf0, 0x82, 0x82, 0xac])); // invalid utf-8 (overlong)
    assert_eq!(" 360", format_item_c(&[0xf0, 0x9f, 0x92])); // invalid utf-8 (missing octet)
    assert_eq!("   \u{10FFFD}", format_item_c(&[0xf4, 0x8f, 0xbf, 0xbd])); // largest valid utf-8   // spell-checker:ignore 10FFFD FFFD
    assert_eq!(" 364", format_item_c(&[0xf4, 0x90, 0x00, 0x00])); // invalid utf-8
    assert_eq!(" 365", format_item_c(&[0xf5, 0x80, 0x80, 0x80])); // invalid utf-8
    assert_eq!(" 377", format_item_c(&[0xff])); // invalid utf-8
}

#[test]
fn test_format_ascii_dump() {
    assert_eq!(">.<", format_ascii_dump(&[0x00]));
    assert_eq!(
        ">. A~.<",
        format_ascii_dump(&[0x1f, 0x20, 0x41, 0x7e, 0x7f])
    );
}
