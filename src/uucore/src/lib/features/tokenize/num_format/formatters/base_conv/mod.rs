// spell-checker:ignore (ToDO) arrnum arr_num mult basenum bufferval refd vals arrfloat conv intermed addl

pub fn arrnum_int_mult(arr_num: &[u8], basenum: u8, base_ten_int_fact: u8) -> Vec<u8> {
    let mut carry: u16 = 0;
    let mut rem: u16;
    let mut new_amount: u16;
    let fact: u16 = u16::from(base_ten_int_fact);
    let base: u16 = u16::from(basenum);

    let mut ret_rev: Vec<u8> = Vec::new();
    let mut it = arr_num.iter().rev();
    loop {
        let i = it.next();
        match i {
            Some(u) => {
                new_amount = (u16::from(*u) * fact) + carry;
                rem = new_amount % base;
                carry = (new_amount - rem) / base;
                ret_rev.push(rem as u8);
            }
            None => {
                while carry != 0 {
                    rem = carry % base;
                    carry = (carry - rem) / base;
                    ret_rev.push(rem as u8);
                }
                break;
            }
        }
    }
    let ret: Vec<u8> = ret_rev.into_iter().rev().collect();
    ret
}

#[allow(dead_code)]
pub struct Remainder<'a> {
    pub position: usize,
    pub replace: Vec<u8>,
    pub arr_num: &'a Vec<u8>,
}

#[allow(dead_code)]
pub struct DivOut<'a> {
    pub quotient: u8,
    pub remainder: Remainder<'a>,
}

#[allow(dead_code)]
pub fn arrnum_int_div_step<'a>(
    rem_in: &'a Remainder,
    radix_in: u8,
    base_ten_int_divisor: u8,
    after_decimal: bool,
) -> DivOut<'a> {
    let mut rem_out = Remainder {
        position: rem_in.position,
        replace: Vec::new(),
        arr_num: rem_in.arr_num,
    };

    let mut bufferval: u16 = 0;
    let base: u16 = u16::from(radix_in);
    let divisor: u16 = u16::from(base_ten_int_divisor);
    let mut traversed = 0;

    let mut quotient = 0;
    let refd_vals = &rem_in.arr_num[rem_in.position + rem_in.replace.len()..];
    let mut it_replace = rem_in.replace.iter();
    let mut it_f = refd_vals.iter();
    loop {
        let u = match it_replace.next() {
            Some(u_rep) => u16::from(*u_rep),
            None => match it_f.next() {
                Some(u_orig) => u16::from(*u_orig),
                None => {
                    if !after_decimal {
                        break;
                    }
                    0
                }
            },
        };
        traversed += 1;
        bufferval += u;
        if bufferval > divisor {
            while bufferval >= divisor {
                quotient += 1;
                bufferval -= divisor;
            }
            rem_out.replace = if bufferval == 0 {
                Vec::new()
            } else {
                let remainder_as_arrnum = unsigned_to_arrnum(bufferval);
                base_conv_vec(&remainder_as_arrnum, 10, radix_in)
            };
            rem_out.position += 1 + (traversed - rem_out.replace.len());
            break;
        } else {
            bufferval *= base;
        }
    }
    DivOut {
        quotient,
        remainder: rem_out,
    }
}
pub fn arrnum_int_add(arrnum: &[u8], basenum: u8, base_ten_int_term: u8) -> Vec<u8> {
    let mut carry: u16 = u16::from(base_ten_int_term);
    let mut rem: u16;
    let mut new_amount: u16;
    let base: u16 = u16::from(basenum);

    let mut ret_rev: Vec<u8> = Vec::new();
    let mut it = arrnum.iter().rev();
    loop {
        let i = it.next();
        match i {
            Some(u) => {
                new_amount = u16::from(*u) + carry;
                rem = new_amount % base;
                carry = (new_amount - rem) / base;
                ret_rev.push(rem as u8);
            }
            None => {
                while carry != 0 {
                    rem = carry % base;
                    carry = (carry - rem) / base;
                    ret_rev.push(rem as u8);
                }
                break;
            }
        }
    }
    let ret: Vec<u8> = ret_rev.into_iter().rev().collect();
    ret
}

pub fn base_conv_vec(src: &[u8], radix_src: u8, radix_dest: u8) -> Vec<u8> {
    let mut result = vec![0];
    for i in src {
        result = arrnum_int_mult(&result, radix_dest, radix_src);
        result = arrnum_int_add(&result, radix_dest, *i);
    }
    result
}

#[allow(dead_code)]
pub fn unsigned_to_arrnum(src: u16) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    let mut src_tmp: u16 = src;
    while src_tmp > 0 {
        result.push((src_tmp % 10) as u8);
        src_tmp /= 10;
    }
    result.reverse();
    result
}

// temporary needs-improvement-function
pub fn base_conv_float(src: &[u8], radix_src: u8, _radix_dest: u8) -> f64 {
    // it would require a lot of addl code
    // to implement this for arbitrary string input.
    // until then, the below operates as an outline
    // of how it would work.
    let mut factor: f64 = 1_f64;
    let radix_src_float: f64 = f64::from(radix_src);
    let mut r: f64 = 0_f64;
    for (i, u) in src.iter().enumerate() {
        if i > 15 {
            break;
        }
        factor /= radix_src_float;
        r += factor * f64::from(*u);
    }
    r
}

pub fn str_to_arrnum(src: &str, radix_def_src: &dyn RadixDef) -> Vec<u8> {
    let mut intermed_in: Vec<u8> = Vec::new();
    for c in src.chars() {
        #[allow(clippy::single_match)]
        match radix_def_src.parse_char(c) {
            Some(u) => {
                intermed_in.push(u);
            }
            None => {} //todo err msg on incorrect
        }
    }
    intermed_in
}

pub fn arrnum_to_str(src: &[u8], radix_def_dest: &dyn RadixDef) -> String {
    let mut str_out = String::new();
    for u in src.iter() {
        #[allow(clippy::single_match)]
        match radix_def_dest.format_u8(*u) {
            Some(c) => {
                str_out.push(c);
            }
            None => {} //todo
        }
    }
    str_out
}

pub fn base_conv_str(
    src: &str,
    radix_def_src: &dyn RadixDef,
    radix_def_dest: &dyn RadixDef,
) -> String {
    let intermed_in: Vec<u8> = str_to_arrnum(src, radix_def_src);
    let intermed_out = base_conv_vec(
        &intermed_in,
        radix_def_src.get_max(),
        radix_def_dest.get_max(),
    );
    arrnum_to_str(&intermed_out, radix_def_dest)
}

pub trait RadixDef {
    fn get_max(&self) -> u8;
    fn parse_char(&self, x: char) -> Option<u8>;
    fn format_u8(&self, x: u8) -> Option<char>;
}
pub struct RadixTen;

const ZERO_ASC: u8 = b'0';
const UPPER_A_ASC: u8 = b'A';
const LOWER_A_ASC: u8 = b'a';

impl RadixDef for RadixTen {
    fn get_max(&self) -> u8 {
        10
    }
    fn parse_char(&self, c: char) -> Option<u8> {
        match c {
            '0'..='9' => Some(c as u8 - ZERO_ASC),
            _ => None,
        }
    }
    fn format_u8(&self, u: u8) -> Option<char> {
        match u {
            0..=9 => Some((ZERO_ASC + u) as char),
            _ => None,
        }
    }
}
pub struct RadixHex;
impl RadixDef for RadixHex {
    fn get_max(&self) -> u8 {
        16
    }
    fn parse_char(&self, c: char) -> Option<u8> {
        match c {
            '0'..='9' => Some(c as u8 - ZERO_ASC),
            'A'..='F' => Some(c as u8 + 10 - UPPER_A_ASC),
            'a'..='f' => Some(c as u8 + 10 - LOWER_A_ASC),
            _ => None,
        }
    }
    fn format_u8(&self, u: u8) -> Option<char> {
        match u {
            0..=9 => Some((ZERO_ASC + u) as char),
            10..=15 => Some((UPPER_A_ASC + (u - 10)) as char),
            _ => None,
        }
    }
}

mod tests;
