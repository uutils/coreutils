#[derive(Copy)]
pub enum FormatWriter {
    IntWriter(fn(u64, usize, usize) -> String),
    FloatWriter(fn(f64) -> String),
    MultibyteWriter(fn(&[u8]) -> String),
}

impl Clone for FormatWriter {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy, Clone)]
pub struct FormatterItemInfo {
    pub byte_size: usize,
    pub print_width: usize,      // including a space in front of the text
    pub formatter: FormatWriter,
}
