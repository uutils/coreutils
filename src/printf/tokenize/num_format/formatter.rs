//! Primitives used by num_format and sub_modules.
//! never dealt with above (e.g. Sub Tokenizer never uses these)

use super::format_field::FormatField;
use cli;
use itertools::{put_back_n, PutBackN};
use std::str::Chars;

// contains the rough ingredients to final
// output for a number, organized together
// to allow for easy generalization of output manipulation
// (e.g. max number of digits after decimal)
pub struct FormatPrimitive {
    pub prefix: Option<String>,
    pub pre_decimal: Option<String>,
    pub post_decimal: Option<String>,
    pub suffix: Option<String>,
}

impl Default for FormatPrimitive {
    fn default() -> FormatPrimitive {
        FormatPrimitive {
            prefix: None,
            pre_decimal: None,
            post_decimal: None,
            suffix: None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Base {
    Ten = 10,
    Hex = 16,
    Octal = 8,
}

// information from the beginning of a numeric argument
// the precedes the beginning of a numeric value
pub struct InPrefix {
    pub radix_in: Base,
    pub sign: i8,
    pub offset: usize,
}

pub trait Formatter {
    //  return a FormatPrimitive for
    // particular field char(s), given the argument
    // string and prefix information (sign, radix)
    fn get_primitive(
        &self,
        field: &FormatField,
        inprefix: &InPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive>;
    // return a string from a formatprimitive,
    // given information about the field
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String;
}
pub fn get_it_at(offset: usize, str_in: &str) -> PutBackN<Chars> {
    put_back_n(str_in[offset..].chars())
}

// TODO: put this somewhere better
pub fn warn_incomplete_conv(pf_arg: &str) {
    // important: keep println here not print
    cli::err_msg(&format!("{}: value not completely converted", pf_arg))
}
