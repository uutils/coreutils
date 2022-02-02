// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety
// spell-checker:ignore (ToDO) arrnum

//! formatter for %a %F C99 Hex-floating-point subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::base_conv;
use super::base_conv::RadixDef;
use super::float_common::{primitive_to_str_common, FloatAnalysis};

#[derive(Default)]
pub struct CninetyNineHexFloatf {
    #[allow(dead_code)]
    as_num: f64,
}
impl CninetyNineHexFloatf {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Formatter for CninetyNineHexFloatf {
    fn get_primitive(
        &self,
        field: &FormatField,
        initial_prefix: &InitialPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        let analysis = FloatAnalysis::analyze(
            str_in,
            initial_prefix,
            Some(second_field as usize),
            None,
            true,
        );
        let f = get_primitive_hex(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            *field.field_char == 'A',
        );
        Some(f)
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}

// c99 hex has unique requirements of all floating point subs in pretty much every part of building a primitive, from prefix and suffix to need for base conversion (in all other cases if you don't have decimal you must have decimal, here it's the other way around)

// on the todo list is to have a trait for get_primitive that is implemented by each float formatter and can override a default. when that happens we can take the parts of get_primitive_dec specific to dec and spin them out to their own functions that can be overridden.
fn get_primitive_hex(
    initial_prefix: &InitialPrefix,
    _str_in: &str,
    _analysis: &FloatAnalysis,
    _last_dec_place: usize,
    capitalized: bool,
) -> FormatPrimitive {
    let prefix = Some(String::from(if initial_prefix.sign == -1 {
        "-0x"
    } else {
        "0x"
    }));

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
    // conversion. The best way to do it is to just convert the float number
    // directly to base 2 and then at the end translate back to hex.
    let mantissa = 0;
    let suffix = Some({
        let ind = if capitalized { "P" } else { "p" };
        if mantissa >= 0 {
            format!("{}+{}", ind, mantissa)
        } else {
            format!("{}{}", ind, mantissa)
        }
    });
    FormatPrimitive {
        prefix,
        suffix,
        ..Default::default()
    }
}

#[allow(dead_code)]
fn to_hex(src: &str, before_decimal: bool) -> String {
    let radix_ten = base_conv::RadixTen;
    let radix_hex = base_conv::RadixHex;
    if before_decimal {
        base_conv::base_conv_str(src, &radix_ten, &radix_hex)
    } else {
        let as_arrnum_ten = base_conv::str_to_arrnum(src, &radix_ten);
        let s = format!(
            "{}",
            base_conv::base_conv_float(&as_arrnum_ten, radix_ten.get_max(), radix_hex.get_max())
        );
        if s.len() > 2 {
            String::from(&s[2..])
        } else {
            // zero
            s
        }
    }
}
