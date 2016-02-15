use super::super::format_field::FormatField;
use super::super::formatter::{InPrefix, Base, FormatPrimitive, warn_incomplete_conv, get_it_at};
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
fn has_enough_digits(hex_input: bool,
                     hex_output: bool,
                     string_position: usize,
                     starting_position: usize,
                     limit: usize)
                     -> bool {
    // -1s are for rounding
    if hex_output {
        if hex_input {
            ((string_position - 1) - starting_position >= limit)
        } else {
            false //undecidable without converting
        }
    } else {
        if hex_input {
            ((((string_position - 1) - starting_position) * 9) / 8 >= limit)
        } else {
            ((string_position - 1) - starting_position >= limit)
        }
    }

}

impl FloatAnalysis {
    pub fn analyze(str_in: &str,
                   inprefix: &InPrefix,
                   max_sd_opt: Option<usize>,
                   max_after_dec_opt: Option<usize>,
                   hex_output: bool)
                   -> FloatAnalysis {
        // this fn assumes
        // the input string
        // has no leading spaces or 0s
        let mut str_it = get_it_at(inprefix.offset, str_in);
        let mut ret = FloatAnalysis {
            len_important: 0,
            decimal_pos: None,
            follow: None,
        };
        let hex_input = match inprefix.radix_in {
            Base::Hex => true,
            Base::Ten => false,
            Base::Octal => {
                panic!("this should never happen: floats should never receive octal input");
            }
        };
        let mut i = 0;
        let mut pos_before_first_nonzero_after_decimal: Option<usize> = None;
        while let Some(c) = str_it.next() {
            match c {
                e @ '0'...'9' | e @ 'A'...'F' | e @ 'a'...'f' => {
                    if !hex_input {
                        match e {
                            '0'...'9' => {}
                            _ => {
                                warn_incomplete_conv(str_in);
                                break;
                            }
                        }
                    }
                    if ret.decimal_pos.is_some() &&
                       pos_before_first_nonzero_after_decimal.is_none() &&
                       e != '0' {
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
    let rten = base_conv::RadixTen;
    let rhex = base_conv::RadixHex;
    if before_decimal {
        base_conv::base_conv_str(src, &rhex, &rten)
    } else {
        let as_arrnum_hex = base_conv::str_to_arrnum(src, &rhex);
        let s = format!("{}",
                        base_conv::base_conv_float(&as_arrnum_hex, rhex.get_max(), rten.get_max()));
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
fn _round_str_from(in_str: &str, position: usize) -> (String, bool) {

    let mut it = in_str[0..position].chars();
    let mut rev = String::new();
    let mut i = position;
    let mut finished_in_dec = false;
    while let Some(c) = it.next_back() {
        i -= 1;
        match c {
            '9' => {
                rev.push('0');
            }
            e @ _ => {
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

fn round_terminal_digit(before_dec: String,
                        after_dec: String,
                        position: usize)
                        -> (String, String) {

    if position < after_dec.len() {
        let digit_at_pos: char;
        {
            digit_at_pos = (&after_dec[position..position + 1])
                               .chars()
                               .next()
                               .expect("");
        }
        match digit_at_pos {
            '5'...'9' => {
                let (new_after_dec, finished_in_dec) = _round_str_from(&after_dec, position);
                if finished_in_dec {
                    return (before_dec, new_after_dec);
                } else {
                    let (new_before_dec, _) = _round_str_from(&before_dec, before_dec.len());
                    return (new_before_dec, new_after_dec);
                }
                // TODO
            }
            _ => {}
        }
    }
    (before_dec, after_dec)
}

pub fn get_primitive_dec(inprefix: &InPrefix,
                         str_in: &str,
                         analysis: &FloatAnalysis,
                         last_dec_place: usize,
                         sci_mode: Option<bool>)
                         -> FormatPrimitive {
    let mut f: FormatPrimitive = Default::default();

    // add negative sign section
    if inprefix.sign == -1 {
        f.prefix = Some(String::from("-"));
    }

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
    // convert to string, de_hexifying if input is in hex.
    let (first_segment, second_segment) = match inprefix.radix_in {
        Base::Hex => {
            (de_hex(first_segment_raw, true),
             de_hex(second_segment_raw, false))
        }
        _ => {
            (String::from(first_segment_raw),
             String::from(second_segment_raw))
        }
    };
    let (pre_dec_unrounded, post_dec_unrounded, mantissa) = if sci_mode.is_some() {
        if first_segment.len() > 1 {
            let mut post_dec = String::from(&first_segment[1..]);
            post_dec.push_str(&second_segment);
            (String::from(&first_segment[0..1]),
             post_dec,
             first_segment.len() as isize - 1)
        } else {
            match first_segment.chars().next() {
                Some('0') => {
                    let mut it = second_segment.chars().enumerate();
                    let mut m: isize = 0;
                    let mut pre = String::from("0");
                    let mut post = String::from("0");
                    while let Some((i, c)) = it.next() {
                        match c {
                            '0' => {}
                            _ => {
                                m = ((i as isize) + 1) * -1;
                                pre = String::from(&second_segment[i..i + 1]);
                                post = String::from(&second_segment[i + 1..]);
                                break;
                            }
                        }
                    }
                    (pre, post, m)
                }
                Some(_) => (first_segment, second_segment, 0),
                None => {
                    panic!("float_common: no chars in first segment.");
                }
            }
        }
    } else {
        (first_segment, second_segment, 0)
    };

    let (pre_dec_draft, post_dec_draft) = round_terminal_digit(pre_dec_unrounded,
                                                               post_dec_unrounded,
                                                               last_dec_place - 1);

    f.pre_decimal = Some(pre_dec_draft);
    f.post_decimal = Some(post_dec_draft);
    if let Some(capitalized) = sci_mode {
        let si_ind = if capitalized {
            'E'
        } else {
            'e'
        };
        f.suffix = Some(if mantissa >= 0 {
            format!("{}+{:02}", si_ind, mantissa)
        } else {
            // negative sign is considered in format!s
            // leading zeroes
            format!("{}{:03}", si_ind, mantissa)
        });
    }

    f
}

pub fn primitive_to_str_common(prim: &FormatPrimitive, field: &FormatField) -> String {
    let mut final_str = String::new();
    match prim.prefix {
        Some(ref prefix) => {
            final_str.push_str(&prefix);
        }
        None => {}
    }
    match prim.pre_decimal {
        Some(ref pre_decimal) => {
            final_str.push_str(&pre_decimal);
        }
        None => {
            panic!("error, format primitives provided to int, will, incidentally under correct \
                    behavior, always have a pre_dec value.");
        }            
    }
    let decimal_places = field.second_field.unwrap_or(6);
    match prim.post_decimal {
        Some(ref post_decimal) => {
            if post_decimal.len() > 0 && decimal_places > 0 {
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
            panic!("error, format primitives provided to int, will, incidentally under correct \
                    behavior, always have a pre_dec value.");
        }
    }
    match prim.suffix {
        Some(ref suffix) => {
            final_str.push_str(suffix);
        }
        None => {}
    }

    final_str
}
