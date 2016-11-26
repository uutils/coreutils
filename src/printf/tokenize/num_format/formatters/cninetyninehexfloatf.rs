//! formatter for %a %F C99 Hex-floating-point subs
use super::super::format_field::FormatField;
use super::super::formatter::{InPrefix, FormatPrimitive, Formatter};
use super::float_common::{FloatAnalysis, primitive_to_str_common};
use super::base_conv;
use super::base_conv::RadixDef;


pub struct CninetyNineHexFloatf {
    as_num: f64,
}
impl CninetyNineHexFloatf {
    pub fn new() -> CninetyNineHexFloatf {
        CninetyNineHexFloatf { as_num: 0.0 }
    }
}

impl Formatter for CninetyNineHexFloatf {
    fn get_primitive(&self,
                     field: &FormatField,
                     inprefix: &InPrefix,
                     str_in: &str)
                     -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        let analysis = FloatAnalysis::analyze(&str_in,
                                              inprefix,
                                              Some(second_field as usize),
                                              None,
                                              true);
        let f = get_primitive_hex(inprefix,
                                  &str_in[inprefix.offset..],
                                  &analysis,
                                  second_field as usize,
                                  *field.field_char == 'A');
        Some(f)
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}

// c99 hex has unique requirements of all floating point subs in pretty much every part of building a primitive, from prefix and suffix to need for base conversion (in all other cases if you don't have decimal you must have decimal, here it's the other way around)

// on the todo list is to have a trait for get_primitive that is implemented by each float formatter and can override a default. when that happens we can take the parts of get_primitive_dec specific to dec and spin them out to their own functions that can be overridden.
#[allow(unused_variables)]
#[allow(unused_assignments)]
fn get_primitive_hex(inprefix: &InPrefix,
                     str_in: &str,
                     analysis: &FloatAnalysis,
                     last_dec_place: usize,
                     capitalized: bool)
                     -> FormatPrimitive {

    let mut f: FormatPrimitive = Default::default();
    f.prefix = Some(String::from(if inprefix.sign == -1 {
        "-0x"
    } else {
        "0x"
    }));

    // assign the digits before and after the decimal points
    // to separate slices. If no digits after decimal point,
    // assign 0
    let (mut first_segment_raw, second_segment_raw) = match analysis.decimal_pos {
        Some(pos) => (&str_in[..pos], &str_in[pos + 1..]),
        None => (&str_in[..], "0"),
    };
    if first_segment_raw.len() == 0 {
        first_segment_raw = "0";
    }
    // convert to string, hexifying if input is in dec.
    // let (first_segment, second_segment) =
    // match inprefix.radix_in {
    // Base::Ten => {
    // (to_hex(first_segment_raw, true),
    // to_hex(second_segment_raw, false))
    // }
    // _ => {
    // (String::from(first_segment_raw),
    // String::from(second_segment_raw))
    // }
    // };
    //
    //
    // f.pre_decimal = Some(first_segment);
    // f.post_decimal = Some(second_segment);
    //

    // TODO actual conversion, make sure to get back mantissa.
    // for hex to hex, it's really just a matter of moving the
    // decimal point and calculating the mantissa by its initial
    // position and its moves, with every position counting for
    // the addition or subtraction of 4 (2**4, because 4 bits in a hex digit)
    // to the exponent.
    // decimal's going to be a little more complicated. correct simulation
    // of glibc will require after-decimal division to a specified precision.
    // the difficult part of this (arrnum_int_div_step) is already implemented.

    // the hex float name may be a bit misleading in terms of how to go about the
    // conversion. The best way to do it is to just convert the floatnum
    // directly to base 2 and then at the end translate back to hex.
    let mantissa = 0;
    f.suffix = Some({
        let ind = if capitalized {
            "P"
        } else {
            "p"
        };
        if mantissa >= 0 {
            format!("{}+{}", ind, mantissa)
        } else {
            format!("{}{}", ind, mantissa)
        }
    });
    f
}

fn to_hex(src: &str, before_decimal: bool) -> String {
    let rten = base_conv::RadixTen;
    let rhex = base_conv::RadixHex;
    if before_decimal {
        base_conv::base_conv_str(src, &rten, &rhex)
    } else {
        let as_arrnum_ten = base_conv::str_to_arrnum(src, &rten);
        let s = format!("{}",
                        base_conv::base_conv_float(&as_arrnum_ten, rten.get_max(), rhex.get_max()));
        if s.len() > 2 {
            String::from(&s[2..])
        } else {
            // zero
            s
        }
    }
}
