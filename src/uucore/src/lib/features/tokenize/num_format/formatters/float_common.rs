// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety
// spell-checker:ignore (ToDO) arrnum

use super::super::format_field::FormatField;
use super::super::formatter::{
    get_it_at, warn_incomplete_conv, Base, FormatPrimitive, InitialPrefix,
};
use super::base_conv;
use super::base_conv::RadixDef;

// if the memory, copy, and comparison cost of chars
//  becomes an issue, we can always operate in vec<u8> here
//  rather than just at de_hex

pub struct FloatAnalysis {
    pub len_important: usize,
    // none means no decimal point.
    pub decimal_pos: Option<usize>,
    pub follow: Option<char>,
}
fn has_enough_digits(
    hex_input: bool,
    hex_output: bool,
    string_position: usize,
    starting_position: usize,
    limit: usize,
) -> bool {
    // -1s are for rounding
    if hex_output {
        if hex_input {
            (string_position - 1) - starting_position >= limit
        } else {
            false //undecidable without converting
        }
    } else if hex_input {
        (((string_position - 1) - starting_position) * 9) / 8 >= limit
    } else {
        (string_position - 1) - starting_position >= limit
    }
}

impl FloatAnalysis {
    pub fn analyze(
        str_in: &str,
        initial_prefix: &InitialPrefix,
        max_sd_opt: Option<usize>,
        max_after_dec_opt: Option<usize>,
        hex_output: bool,
    ) -> Self {
        // this fn assumes
        // the input string
        // has no leading spaces or 0s
        let str_it = get_it_at(initial_prefix.offset, str_in);
        let mut ret = Self {
            len_important: 0,
            decimal_pos: None,
            follow: None,
        };
        let hex_input = match initial_prefix.radix_in {
            Base::Hex => true,
            Base::Ten => false,
            Base::Octal => {
                panic!("this should never happen: floats should never receive octal input");
            }
        };
        let mut i = 0;
        let mut pos_before_first_nonzero_after_decimal: Option<usize> = None;
        for c in str_it {
            match c {
                e @ '0'..='9' | e @ 'A'..='F' | e @ 'a'..='f' => {
                    if !hex_input {
                        match e {
                            '0'..='9' => {}
                            _ => {
                                warn_incomplete_conv(str_in);
                                break;
                            }
                        }
                    }
                    if ret.decimal_pos.is_some()
                        && pos_before_first_nonzero_after_decimal.is_none()
                        && e != '0'
                    {
                        pos_before_first_nonzero_after_decimal = Some(i - 1);
                    }
                    if let Some(max_sd) = max_sd_opt {
                        if i == max_sd {
                            // follow is used in cases of %g
                            // where the character right after the last
                            // sd is considered is rounded affecting
                            // the previous digit in 1/2 of instances
                            ret.follow = Some(e);
                        } else if ret.decimal_pos.is_some() && i > max_sd {
                            break;
                        }
                    }
                    if let Some(max_after_dec) = max_after_dec_opt {
                        if let Some(p) = ret.decimal_pos {
                            if has_enough_digits(hex_input, hex_output, i, p, max_after_dec) {
                                break;
                            }
                        }
                    } else if let Some(max_sd) = max_sd_opt {
                        if let Some(p) = pos_before_first_nonzero_after_decimal {
                            if has_enough_digits(hex_input, hex_output, i, p, max_sd) {
                                break;
                            }
                        }
                    }
                }
                '.' => {
                    if ret.decimal_pos.is_none() {
                        ret.decimal_pos = Some(i);
                    } else {
                        warn_incomplete_conv(str_in);
                        break;
                    }
                }
                _ => {
                    warn_incomplete_conv(str_in);
                    break;
                }
            };
            i += 1;
        }
        ret.len_important = i;
        ret
    }
}

fn de_hex(src: &str, before_decimal: bool) -> String {
    let radix_ten = base_conv::RadixTen;
    let radix_hex = base_conv::RadixHex;
    if before_decimal {
        base_conv::base_conv_str(src, &radix_hex, &radix_ten)
    } else {
        let as_arrnum_hex = base_conv::str_to_arrnum(src, &radix_hex);
        let s = format!(
            "{}",
            base_conv::base_conv_float(&as_arrnum_hex, radix_hex.get_max(), radix_ten.get_max())
        );
        if s.len() > 2 {
            String::from(&s[2..])
        } else {
            // zero
            s
        }
    }
}

// takes a string in,
// truncates to a position,
// bumps the last digit up one,
// and if the digit was nine
// propagate to the next, etc.
// If before the decimal and the most
// significant digit is a 9, it becomes a 1
fn _round_str_from(in_str: &str, position: usize, before_dec: bool) -> (String, bool) {
    let mut it = in_str[0..position].chars();
    let mut rev = String::new();
    let mut i = position;
    let mut finished_in_dec = false;
    while let Some(c) = it.next_back() {
        i -= 1;
        match c {
            '9' => {
                // If we're before the decimal
                // and on the most significant digit,
                // round 9 to 1, else to 0.
                if before_dec && i == 0 {
                    rev.push('1');
                } else {
                    rev.push('0');
                }
            }
            e => {
                rev.push(((e as u8) + 1) as char);
                finished_in_dec = true;
                break;
            }
        }
    }
    let mut fwd = String::from(&in_str[0..i]);
    for ch in rev.chars().rev() {
        fwd.push(ch);
    }
    (fwd, finished_in_dec)
}

fn round_terminal_digit(
    before_dec: String,
    after_dec: String,
    position: usize,
) -> (String, String, bool) {
    if position < after_dec.len() {
        let digit_at_pos: char;
        {
            digit_at_pos = after_dec[position..=position].chars().next().expect("");
        }
        if let '5'..='9' = digit_at_pos {
            let (new_after_dec, finished_in_dec) = _round_str_from(&after_dec, position, false);
            if finished_in_dec {
                return (before_dec, new_after_dec, false);
            } else {
                let (new_before_dec, _) = _round_str_from(&before_dec, before_dec.len(), true);
                let mut dec_place_chg = false;
                let mut before_dec_chars = new_before_dec.chars();
                if before_dec_chars.next() == Some('1') && before_dec_chars.all(|c| c == '0') {
                    // If the first digit is a one and remaining are zeros, we have
                    // rounded to a new decimal place, so the decimal place must be updated.
                    // Only update decimal place if the before decimal != 0
                    dec_place_chg = before_dec != "0";
                }
                return (new_before_dec, new_after_dec, dec_place_chg);
            }
            // TODO
        }
    }
    (before_dec, after_dec, false)
}

pub fn get_primitive_dec(
    initial_prefix: &InitialPrefix,
    str_in: &str,
    analysis: &FloatAnalysis,
    last_dec_place: usize,
    sci_mode: Option<bool>,
) -> FormatPrimitive {
    let mut f: FormatPrimitive = Default::default();

    // add negative sign section
    if initial_prefix.sign == -1 {
        f.prefix = Some(String::from("-"));
    }

    // assign the digits before and after the decimal points
    // to separate slices. If no digits after decimal point,
    // assign 0
    let (mut first_segment_raw, second_segment_raw) = match analysis.decimal_pos {
        Some(pos) => (&str_in[..pos], &str_in[pos + 1..]),
        None => (str_in, "0"),
    };
    if first_segment_raw.is_empty() {
        first_segment_raw = "0";
    }
    // convert to string, de_hexifying if input is in hex   // spell-checker:disable-line
    let (first_segment, second_segment) = match initial_prefix.radix_in {
        Base::Hex => (
            de_hex(first_segment_raw, true),
            de_hex(second_segment_raw, false),
        ),
        _ => (
            String::from(first_segment_raw),
            String::from(second_segment_raw),
        ),
    };
    let (pre_dec_unrounded, post_dec_unrounded, mut mantissa) = if sci_mode.is_some() {
        if first_segment.len() > 1 {
            let mut post_dec = String::from(&first_segment[1..]);
            post_dec.push_str(&second_segment);
            (
                String::from(&first_segment[0..1]),
                post_dec,
                first_segment.len() as isize - 1,
            )
        } else {
            match first_segment
                .chars()
                .next()
                .expect("float_common: no chars in first segment.")
            {
                '0' => {
                    let it = second_segment.chars().enumerate();
                    let mut m: isize = 0;
                    let mut pre = String::from("0");
                    let mut post = String::from("0");
                    for (i, c) in it {
                        match c {
                            '0' => {}
                            _ => {
                                m = -((i as isize) + 1);
                                pre = String::from(&second_segment[i..=i]);
                                post = String::from(&second_segment[i + 1..]);
                                break;
                            }
                        }
                    }
                    (pre, post, m)
                }
                _ => (first_segment, second_segment, 0),
            }
        }
    } else {
        (first_segment, second_segment, 0)
    };

    let (pre_dec_draft, post_dec_draft, dec_place_chg) =
        round_terminal_digit(pre_dec_unrounded, post_dec_unrounded, last_dec_place - 1);
    f.post_decimal = Some(post_dec_draft);
    if let Some(capitalized) = sci_mode {
        let si_ind = if capitalized { 'E' } else { 'e' };
        // Increase the mantissa if we're adding a decimal place
        if dec_place_chg {
            mantissa += 1;
        }
        f.suffix = Some(if mantissa >= 0 {
            format!("{}+{:02}", si_ind, mantissa)
        } else {
            // negative sign is considered in format!s
            // leading zeroes
            format!("{}{:03}", si_ind, mantissa)
        });
        f.pre_decimal = Some(pre_dec_draft);
    } else if dec_place_chg {
        // We've rounded up to a new decimal place so append 0
        f.pre_decimal = Some(pre_dec_draft + "0");
    } else {
        f.pre_decimal = Some(pre_dec_draft);
    }

    f
}

pub fn primitive_to_str_common(prim: &FormatPrimitive, field: &FormatField) -> String {
    let mut final_str = String::new();
    if let Some(ref prefix) = prim.prefix {
        final_str.push_str(prefix);
    }
    match prim.pre_decimal {
        Some(ref pre_decimal) => {
            final_str.push_str(pre_decimal);
        }
        None => {
            panic!(
                "error, format primitives provided to int, will, incidentally under correct \
                 behavior, always have a pre_dec value."
            );
        }
    }
    let decimal_places = field.second_field.unwrap_or(6);
    match prim.post_decimal {
        Some(ref post_decimal) => {
            if !post_decimal.is_empty() && decimal_places > 0 {
                final_str.push('.');
                let len_avail = post_decimal.len() as u32;

                if decimal_places >= len_avail {
                    // println!("dec {}, len avail {}", decimal_places, len_avail);
                    final_str.push_str(post_decimal);

                    if *field.field_char != 'g' && *field.field_char != 'G' {
                        let diff = decimal_places - len_avail;
                        for _ in 0..diff {
                            final_str.push('0');
                        }
                    }
                } else {
                    // println!("printing to only {}", decimal_places);
                    final_str.push_str(&post_decimal[0..decimal_places as usize]);
                }
            }
        }
        None => {
            panic!(
                "error, format primitives provided to int, will, incidentally under correct \
                 behavior, always have a pre_dec value."
            );
        }
    }
    if let Some(ref suffix) = prim.suffix {
        final_str.push_str(suffix);
    }

    final_str
}
