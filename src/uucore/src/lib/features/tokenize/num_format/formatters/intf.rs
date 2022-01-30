// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety
// spell-checker:ignore (ToDO) arrnum

//! formatter for unsigned and signed int subs
//! unsigned int: %X %x (hex u64) %o (octal u64) %u (base ten u64)
//! signed int: %i %d (both base ten i64)
use super::super::format_field::FormatField;
use super::super::formatter::{
    get_it_at, warn_incomplete_conv, Base, FormatPrimitive, Formatter, InitialPrefix,
};
use std::i64;
use std::u64;

#[derive(Default)]
pub struct Intf {
    _a: u32,
}

// see the Intf::analyze() function below
struct IntAnalysis {
    check_past_max: bool,
    past_max: bool,
    is_zero: bool,
    len_digits: u8,
}

impl Intf {
    pub fn new() -> Self {
        Self::default()
    }
    // take a ref to argument string, and basic information
    // about prefix (offset, radix, sign), and analyze string
    // to gain the IntAnalysis information above
    // check_past_max: true if the number *may* be above max,
    //   but we don't know either way. One of several reasons
    //   we may have to parse as int.
    // past_max: true if the object is past max, false if not
    //  in the future we should probably combine these into an
    //  Option<bool>
    // is_zero: true if number is zero, false otherwise
    // len_digits: length of digits used to create the int
    //   important, for example, if we run into a non-valid character
    fn analyze(str_in: &str, signed_out: bool, initial_prefix: &InitialPrefix) -> IntAnalysis {
        // the maximum number of digits we could conceivably
        // have before the decimal point without exceeding the
        // max
        let mut str_it = get_it_at(initial_prefix.offset, str_in);
        let max_sd_in = if signed_out {
            match initial_prefix.radix_in {
                Base::Ten => 19,
                Base::Octal => 21,
                Base::Hex => 16,
            }
        } else {
            match initial_prefix.radix_in {
                Base::Ten => 20,
                Base::Octal => 22,
                Base::Hex => 16,
            }
        };
        let mut ret = IntAnalysis {
            check_past_max: false,
            past_max: false,
            is_zero: false,
            len_digits: 0,
        };

        // todo turn this to a while let now that we know
        // no special behavior on EOI break
        loop {
            let c_opt = str_it.next();
            if let Some(c) = c_opt {
                match c {
                    '0'..='9' | 'a'..='f' | 'A'..='F' => {
                        if ret.len_digits == 0 && c == '0' {
                            ret.is_zero = true;
                        } else if ret.is_zero {
                            ret.is_zero = false;
                        }
                        ret.len_digits += 1;
                        if ret.len_digits == max_sd_in {
                            if let Some(next_ch) = str_it.next() {
                                match next_ch {
                                    '0'..='9' => {
                                        ret.past_max = true;
                                    }
                                    _ => {
                                        // force conversion
                                        // to check if its above max.
                                        // todo: spin out convert
                                        // into fn, call it here to try
                                        // read val, on Ok()
                                        // save val for reuse later
                                        // that way on same-base in and out
                                        // we don't needlessly convert int
                                        // to str, we can just copy it over.
                                        ret.check_past_max = true;
                                        str_it.put_back(next_ch);
                                    }
                                }
                                if ret.past_max {
                                    break;
                                }
                            } else {
                                ret.check_past_max = true;
                            }
                        }
                    }
                    _ => {
                        warn_incomplete_conv(str_in);
                        break;
                    }
                }
            } else {
                // breaks on EOL
                break;
            }
        }
        ret
    }
    // get a FormatPrimitive of the maximum value for the field char
    //  and given sign
    fn get_max(field_char: char, sign: i8) -> FormatPrimitive {
        let mut fmt_primitive: FormatPrimitive = Default::default();
        fmt_primitive.pre_decimal = Some(String::from(match field_char {
            'd' | 'i' => match sign {
                1 => "9223372036854775807",
                _ => {
                    fmt_primitive.prefix = Some(String::from("-"));
                    "9223372036854775808"
                }
            },
            'x' | 'X' => "ffffffffffffffff",
            'o' => "1777777777777777777777",
            /* 'u' | */ _ => "18446744073709551615",
        }));
        fmt_primitive
    }
    // conv_from_segment contract:
    // 1. takes
    // - a string that begins with a non-zero digit, and proceeds
    //  with zero or more following digits until the end of the string
    // - a radix to interpret those digits as
    // - a char that communicates:
    //     whether to interpret+output the string as an i64 or u64
    //     what radix to write the parsed number as.
    // 2. parses it as a rust integral type
    // 3. outputs FormatPrimitive with:
    // - if the string falls within bounds:
    //   number parsed and written in the correct radix
    // - if the string falls outside bounds:
    //   for i64 output, the int minimum or int max (depending on sign)
    //   for u64 output, the u64 max in the output radix
    fn conv_from_segment(
        segment: &str,
        radix_in: Base,
        field_char: char,
        sign: i8,
    ) -> FormatPrimitive {
        match field_char {
            'i' | 'd' => match i64::from_str_radix(segment, radix_in as u32) {
                Ok(i) => {
                    let mut fmt_prim: FormatPrimitive = Default::default();
                    if sign == -1 {
                        fmt_prim.prefix = Some(String::from("-"));
                    }
                    fmt_prim.pre_decimal = Some(format!("{}", i));
                    fmt_prim
                }
                Err(_) => Self::get_max(field_char, sign),
            },
            _ => match u64::from_str_radix(segment, radix_in as u32) {
                Ok(u) => {
                    let mut fmt_prim: FormatPrimitive = Default::default();
                    let u_f = if sign == -1 { u64::MAX - (u - 1) } else { u };
                    fmt_prim.pre_decimal = Some(match field_char {
                        'X' => format!("{:X}", u_f),
                        'x' => format!("{:x}", u_f),
                        'o' => format!("{:o}", u_f),
                        _ => format!("{}", u_f),
                    });
                    fmt_prim
                }
                Err(_) => Self::get_max(field_char, sign),
            },
        }
    }
}
impl Formatter for Intf {
    fn get_primitive(
        &self,
        field: &FormatField,
        initial_prefix: &InitialPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive> {
        let begin = initial_prefix.offset;

        // get information about the string. see Intf::Analyze
        // def above.
        let convert_hints = Self::analyze(
            str_in,
            *field.field_char == 'i' || *field.field_char == 'd',
            initial_prefix,
        );
        // We always will have a format primitive to return
        Some(if convert_hints.len_digits == 0 || convert_hints.is_zero {
            // if non-digit or end is reached before a non-zero digit
            FormatPrimitive {
                pre_decimal: Some(String::from("0")),
                ..Default::default()
            }
        } else if !convert_hints.past_max {
            // if the number is or may be below the bounds limit
            let radix_out = match *field.field_char {
                'd' | 'i' | 'u' => Base::Ten,
                'x' | 'X' => Base::Hex,
                /* 'o' | */ _ => Base::Octal,
            };
            let radix_mismatch = !radix_out.eq(&initial_prefix.radix_in);
            let decrease_from_max: bool = initial_prefix.sign == -1 && *field.field_char != 'i';
            let end = begin + convert_hints.len_digits as usize;

            // convert to int if any one of these is true:
            // - number of digits in int indicates it may be past max
            // - we're subtracting from the max
            // - we're converting the base
            if convert_hints.check_past_max || decrease_from_max || radix_mismatch {
                // radix of in and out is the same.
                let segment = String::from(&str_in[begin..end]);
                Self::conv_from_segment(
                    &segment,
                    initial_prefix.radix_in.clone(),
                    *field.field_char,
                    initial_prefix.sign,
                )
            } else {
                // otherwise just do a straight string copy.
                let mut fmt_prim: FormatPrimitive = Default::default();

                // this is here and not earlier because
                // zero doesn't get a sign, and conv_from_segment
                // creates its format primitive separately
                if initial_prefix.sign == -1 && *field.field_char == 'i' {
                    fmt_prim.prefix = Some(String::from("-"));
                }
                fmt_prim.pre_decimal = Some(String::from(&str_in[begin..end]));
                fmt_prim
            }
        } else {
            Self::get_max(*field.field_char, initial_prefix.sign)
        })
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        let mut final_str: String = String::new();
        if let Some(ref prefix) = prim.prefix {
            final_str.push_str(prefix);
        }
        // integral second fields is zero-padded minimum-width
        // which gets handled before general minimum-width
        match prim.pre_decimal {
            Some(ref pre_decimal) => {
                if let Some(min) = field.second_field {
                    let mut i = min;
                    let len = pre_decimal.len() as u32;
                    while i > len {
                        final_str.push('0');
                        i -= 1;
                    }
                }
                final_str.push_str(pre_decimal);
            }
            None => {
                panic!(
                    "error, format primitives provided to int, will, incidentally under \
                     correct behavior, always have a pre_dec value."
                );
            }
        }
        final_str
    }
}
