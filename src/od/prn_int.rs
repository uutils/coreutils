use formatteriteminfo::*;

pub static FORMAT_ITEM_OCT8: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 4, // max: 377
    formatter: FormatWriter::IntWriter(format_item_oct),
};

pub static FORMAT_ITEM_OCT16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 7, // max: 177777
    formatter: FormatWriter::IntWriter(format_item_oct),
};

pub static FORMAT_ITEM_OCT32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 12, // max: 37777777777
    formatter: FormatWriter::IntWriter(format_item_oct),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_OCT64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 23, // max: 2000000000000000000000
    formatter: FormatWriter::IntWriter(format_item_oct),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_HEX8: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 3, // max: ff
    formatter: FormatWriter::IntWriter(format_item_hex),
};

pub static FORMAT_ITEM_HEX16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 5, // max: ffff
    formatter: FormatWriter::IntWriter(format_item_hex),
};

pub static FORMAT_ITEM_HEX32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 9, // max: ffffffff
    formatter: FormatWriter::IntWriter(format_item_hex),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_HEX64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 17, // max: ffffffffffffffff
    formatter: FormatWriter::IntWriter(format_item_hex),
};


#[allow(dead_code)]
pub static FORMAT_ITEM_DEC8U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 4, // max: 255
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

pub static FORMAT_ITEM_DEC16U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 6, // max: 65535
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

pub static FORMAT_ITEM_DEC32U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 11, // max: 4294967295
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC64U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 21, // max:  18446744073709551615
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};


#[allow(dead_code)]
pub static FORMAT_ITEM_DEC8S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 5, // max: -128
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

pub static FORMAT_ITEM_DEC16S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 7, // max: -32768
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC32S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 12, // max: -2147483648
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC64S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 21, // max: -9223372036854775808
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};


// TODO: use some sort of byte iterator, instead of passing bytes in u64
pub fn format_item_oct(p: u64, _: usize, print_width: usize) -> String {

    format!(" {:0width$o}",
           p,
           width = print_width - 1)
}

pub fn format_item_hex(p: u64, _: usize, print_width: usize) -> String {

    format!(" {:0width$x}",
           p,
           width = print_width - 1)
}


fn sign_extend(item: u64, itembytes: usize) -> i64{
    let shift = 64 - itembytes * 8;
    (item << shift) as i64 >> shift
}


pub fn format_item_dec_s(p: u64, itembytes: usize, print_width: usize) -> String {
    // sign extend
    let s = sign_extend(p, itembytes);
    format!("{:width$}", s, width = print_width)
}

pub fn format_item_dec_u(p: u64, _: usize, print_width: usize) -> String {
    format!("{:width$}", p, width = print_width)
}
