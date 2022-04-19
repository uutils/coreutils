// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

//! Primitives used by Sub Tokenizer
//! and num_format modules
#[derive(Clone)]
pub enum FieldType {
    Strf,
    Floatf,
    CninetyNineHexFloatf,
    Scif,
    Decf,
    Intf,
    Charf,
}

// #[allow(non_camel_case_types)]
// pub enum FChar {
// d,
// e,
// E,
// i,
// f,
// F,
// g,
// G,
// u,
// x,
// X,
// o
// }
//

// a Sub Tokens' fields are stored
// as a single object so they can be more simply
// passed by ref to num_format in a Sub method
#[derive(Clone)]
pub struct FormatField<'a> {
    pub min_width: Option<isize>,
    pub second_field: Option<u32>,
    pub field_char: &'a char,
    pub field_type: &'a FieldType,
    pub orig: &'a String,
}
