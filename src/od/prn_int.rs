use formatteriteminfo::*;

pub static FORMAT_ITEM_OCT8: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 3,
    formatter: FormatWriter::IntWriter(format_item_oct),
};

pub static FORMAT_ITEM_OCT16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 6,
    formatter: FormatWriter::IntWriter(format_item_oct),
};

pub static FORMAT_ITEM_OCT32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 12,
    formatter: FormatWriter::IntWriter(format_item_oct),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_OCT64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 24,
    formatter: FormatWriter::IntWriter(format_item_oct),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_HEX8: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 2,
    formatter: FormatWriter::IntWriter(format_item_hex),
};

pub static FORMAT_ITEM_HEX16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 4,
    formatter: FormatWriter::IntWriter(format_item_hex),
};

pub static FORMAT_ITEM_HEX32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 8,
    formatter: FormatWriter::IntWriter(format_item_hex),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_HEX64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 16,
    formatter: FormatWriter::IntWriter(format_item_hex),
};


#[allow(dead_code)]
pub static FORMAT_ITEM_DEC8U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 3,
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

pub static FORMAT_ITEM_DEC16U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 5,
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

pub static FORMAT_ITEM_DEC32U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 10,
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC64U: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 19,
    formatter: FormatWriter::IntWriter(format_item_dec_u),
};


#[allow(dead_code)]
pub static FORMAT_ITEM_DEC8S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 1,
    print_width: 4,
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

pub static FORMAT_ITEM_DEC16S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 6,
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC32S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 11,
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};

#[allow(dead_code)]
pub static FORMAT_ITEM_DEC64S: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 20,
    formatter: FormatWriter::IntWriter(format_item_dec_s),
};


// TODO: use some sort of byte iterator, instead of passing bytes in u64
pub fn format_item_oct(p: u64, _: usize, print_width: usize) -> String {

    format!(" {:0width$o}",
           p,
           width = print_width)
}

pub fn format_item_hex(p: u64, _: usize, print_width: usize) -> String {

    format!(" {:0width$x}",
           p,
           width = print_width)
}


fn sign_extend(item: u64, itembytes: usize) -> i64{
    let shift = 64 - itembytes * 8;
    (item << shift) as i64 >> shift
}


pub fn format_item_dec_s(p: u64, itembytes: usize, print_width: usize) -> String {
    // sign extend
    let s = sign_extend(p, itembytes);
    format!(" {:width$}", s, width = print_width)
}

pub fn format_item_dec_u(p: u64, _: usize, print_width: usize) -> String {
    format!(" {:width$}", p, width = print_width)
}
