#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FormatWriter {
    IntWriter(fn(u64, usize, usize) -> String),
    FloatWriter(fn(f64) -> String),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FormatterItemInfo {
    pub byte_size: usize,
    pub print_width: usize,      // including a space in front of the text
    pub formatter: FormatWriter,
}
