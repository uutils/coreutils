// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore langinfo charmap eucjp euckr euctw CTYPE HKSCS hkscs localedata

//! Locale-aware multi-byte character length detection via `LC_CTYPE`.

use std::sync::OnceLock;

enum MbEncoding {
    Utf8,
    Gb18030,
    EucJp,
    EucKr,
    Big5,
}

fn encoding_from_name(enc: &str) -> MbEncoding {
    match enc {
        "gb18030" | "gbk" | "gb2312" => MbEncoding::Gb18030,
        "euc-jp" | "eucjp" => MbEncoding::EucJp,
        "euc-kr" | "euckr" => MbEncoding::EucKr,
        "big5" | "big5-hkscs" | "big5hkscs" | "euc-tw" | "euctw" => MbEncoding::Big5,
        _ => MbEncoding::Utf8,
    }
}

fn get_encoding() -> &'static MbEncoding {
    static ENCODING: OnceLock<MbEncoding> = OnceLock::new();
    ENCODING.get_or_init(|| {
        let val = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|&k| std::env::var(k).ok().filter(|v| !v.is_empty()));
        let s = match val.as_deref() {
            Some(s) if s != "C" && s != "POSIX" => s,
            _ => return MbEncoding::Utf8,
        };
        if let Some(enc) = s.split('.').nth(1) {
            let enc = enc.split('@').next().unwrap_or(enc);
            encoding_from_name(&enc.to_ascii_lowercase())
        } else {
            // Bare locale defaults from glibc localedata/SUPPORTED
            match s.split('@').next().unwrap_or(s) {
                "zh_CN" | "zh_SG" => MbEncoding::Gb18030,
                "zh_TW" | "zh_HK" => MbEncoding::Big5,
                _ => MbEncoding::Utf8,
            }
        }
    })
}

/// Byte length of the first character in `bytes` under the current locale encoding.
/// Returns 1 for empty, invalid, or incomplete sequences.
pub fn mb_char_len(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 1;
    }
    let b0 = bytes[0];
    if b0 <= 0x7F {
        return 1;
    }
    match get_encoding() {
        MbEncoding::Utf8 => utf8_len(bytes, b0),
        MbEncoding::Gb18030 => gb18030_len(bytes, b0),
        MbEncoding::EucJp => eucjp_len(bytes, b0),
        MbEncoding::EucKr => euckr_len(bytes, b0),
        MbEncoding::Big5 => big5_len(bytes, b0),
    }
}

// All helpers below assume b0 > 0x7F (ASCII already handled by caller).

fn utf8_len(b: &[u8], b0: u8) -> usize {
    let n = match b0 {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return 1,
    };
    if b.len() >= n && b[1..n].iter().all(|&c| c & 0xC0 == 0x80) {
        n
    } else {
        1
    }
}

// 2-byte: [81-FE][40-7E,80-FE]  4-byte: [81-FE][30-39][81-FE][30-39]
fn gb18030_len(b: &[u8], b0: u8) -> usize {
    if !(0x81..=0xFE).contains(&b0) {
        return 1;
    }
    if b.len() >= 4
        && (0x30..=0x39).contains(&b[1])
        && (0x81..=0xFE).contains(&b[2])
        && (0x30..=0x39).contains(&b[3])
    {
        return 4;
    }
    if b.len() >= 2 && ((0x40..=0x7E).contains(&b[1]) || (0x80..=0xFE).contains(&b[1])) {
        return 2;
    }
    1
}

// 3-byte: [8F][A1-FE][A1-FE]  2-byte: [8E][A1-DF] or [A1-FE][A1-FE]
fn eucjp_len(b: &[u8], b0: u8) -> usize {
    if b0 == 0x8F && b.len() >= 3 && (0xA1..=0xFE).contains(&b[1]) && (0xA1..=0xFE).contains(&b[2])
    {
        return 3;
    }
    if b.len() >= 2 {
        if b0 == 0x8E && (0xA1..=0xDF).contains(&b[1]) {
            return 2;
        }
        if (0xA1..=0xFE).contains(&b0) && (0xA1..=0xFE).contains(&b[1]) {
            return 2;
        }
    }
    1
}

// 2-byte: [A1-FE][A1-FE]
fn euckr_len(b: &[u8], b0: u8) -> usize {
    if (0xA1..=0xFE).contains(&b0) && b.len() >= 2 && (0xA1..=0xFE).contains(&b[1]) {
        2
    } else {
        1
    }
}

// 2-byte: [81-FE][40-7E,A1-FE]
fn big5_len(b: &[u8], b0: u8) -> usize {
    if (0x81..=0xFE).contains(&b0)
        && b.len() >= 2
        && ((0x40..=0x7E).contains(&b[1]) || (0xA1..=0xFE).contains(&b[1]))
    {
        2
    } else {
        1
    }
}
