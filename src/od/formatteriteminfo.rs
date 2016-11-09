use std::fmt;

#[derive(Copy)]
pub enum FormatWriter {
    IntWriter(fn(u64) -> String),
    FloatWriter(fn(f64) -> String),
    MultibyteWriter(fn(&[u8]) -> String),
}

impl Clone for FormatWriter {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for FormatWriter {
    fn eq(&self, other: &FormatWriter) -> bool {
        use formatteriteminfo::FormatWriter::*;

        match (self, other) {
            (&IntWriter(ref a), &IntWriter(ref b)) => a == b,
            (&FloatWriter(ref a), &FloatWriter(ref b)) => a == b,
            (&MultibyteWriter(ref a), &MultibyteWriter(ref b)) => *a as usize == *b as usize,
            _ => false,
        }
    }
}

impl Eq for FormatWriter {}

impl fmt::Debug for FormatWriter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &FormatWriter::IntWriter(ref p) => {
                try!(f.write_str("IntWriter:"));
                fmt::Pointer::fmt(p, f)
            },
            &FormatWriter::FloatWriter(ref p) => {
                try!(f.write_str("FloatWriter:"));
                fmt::Pointer::fmt(p, f)
            },
            &FormatWriter::MultibyteWriter(ref p) => {
                try!(f.write_str("MultibyteWriter:"));
                fmt::Pointer::fmt(&(*p as *const ()), f)
            },
        }
     }
 }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FormatterItemInfo {
    pub byte_size: usize,
    pub print_width: usize,      // including a space in front of the text
    pub formatter: FormatWriter,
}
