pub fn arrnum_int_mult(
    arrnum : &Vec<u8>,
    basenum : u8,
    base_ten_int_fact : u8 
    ) -> Vec<u8> {
    let mut carry : u16 = 0;
    let mut rem : u16;
    let mut new_amount : u16;
    let fact : u16 = base_ten_int_fact as u16;
    let base : u16 = basenum as u16;
     
    let mut ret_rev : Vec<u8> = Vec::new();
    let mut it = arrnum.iter().rev();
    loop {
        let i = it.next();
        match i {
            Some(u) => {
                new_amount = ((u.clone() as u16)*fact) + carry;
                rem = new_amount % base;
                carry = (new_amount - rem) / base;
                ret_rev.push(rem as u8)
            },
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
    let ret : Vec<u8> =
        ret_rev.iter().rev().map(|x| x.clone()).collect();
    ret
}

pub struct Remainder {    
    position : usize,
    replace : Option<u8>
}

pub struct DivOut {
    quotient : u8,
    remainder: Remainder
}

pub fn arrnum_int_div(
    arrnum : &Vec<u8>,
    basenum : u8,
    base_ten_int_divisor : u8,
    rem_in : Remainder
        ) -> DivOut  {

    let mut rem_out = Remainder {
        position: rem_in.position,
        replace : None
    };

    let mut bufferval : u16 = 0;
    let base : u16 = basenum as u16;
    let divisor : u16 = base_ten_int_divisor as u16;

    let mut quotient = 0;
    let mut u_cur : Option<&u8> = Some(match rem_in.replace {
        Some(ref u) => { u }
        None => { &arrnum[rem_in.position] }
    });

    let str_f = &arrnum[rem_in.position+1..];
    let mut it_f = str_f.iter();
    loop {        
        match u_cur {
            Some(u) => {
                bufferval += u.clone() as u16;
                if bufferval > divisor {
                    while bufferval >= divisor {
                        quotient+=1;
                        bufferval -= divisor;
                    }
                    if bufferval == 0 {
                        rem_out.position +=1;
                    } else {
                        rem_out.replace = Some(bufferval as u8);
                    }
                    break;
                } else {
                    bufferval *= base;                    
                }                
            },
            None => {
                break;
            }
        }
        u_cur = it_f.next().clone();
        rem_out.position+=1;
    }
    DivOut { quotient: quotient, remainder: rem_out }
}

pub fn arrnum_int_add(
    arrnum : &Vec<u8>,
    basenum : u8,
    base_ten_int_term : u8 
    ) -> Vec<u8> {
    let mut carry : u16 = base_ten_int_term as u16;
    let mut rem : u16;
    let mut new_amount : u16;
    let base : u16 = basenum as u16;
    
    let mut ret_rev : Vec<u8> = Vec::new();
    let mut it = arrnum.iter().rev();
    loop {
        let i = it.next();
        match i {            
            Some(u) => {
                new_amount = (u.clone() as u16) + carry;
                rem = new_amount % base;
                carry = (new_amount - rem) / base;
                ret_rev.push(rem as u8)
            },
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
    let ret : Vec<u8> =
        ret_rev.iter().rev().map(|x| x.clone()).collect();
    ret
}

pub fn base_conv_vec(
    src : &Vec<u8>,
    radix_src : u8,
    radix_dest : u8
        ) -> Vec<u8> {
    let mut result : Vec<u8> = Vec::new();
    result.push(0);
    for i in src {
        result = arrnum_int_mult(&result,
                                     radix_dest, radix_src);
        result = arrnum_int_add(
            &result,
            radix_dest,
            i.clone()
            );
    }
    result
}


pub fn base_conv_float(
    src : &Vec<u8>,
    radix_src : u8,
    radix_dest : u8
        ) -> f64 {
    //it would require a lot of addl code
    // to implement this for arbitrary string input.
    //until then, the below operates as an outline
    // of how it would work.
    let mut result : Vec<u8> = Vec::new();
    result.push(0);
    let mut factor : f64 = radix_dest as f64;
    let radix_src_float : f64 = radix_src as f64;
    let mut i = 0;
    let mut r :f64 = 0 as f64;
    factor /= 10.;
    for u in src {
        if i > 15 { break; }
        i+=1;
        factor /= radix_src_float;
        r += factor * (u.clone() as f64)
    }
    r
}

pub fn str_to_arrnum(
    src: &str,
    radix_def_src : &RadixDef
        ) -> Vec<u8> {
    let mut intermed_in : Vec<u8> = Vec::new();
    for c in src.chars() {
        match radix_def_src.from_char::<>(c) {
            Some(u) => { intermed_in.push(u); }
            None => {} //todo err msg on incorrect
        }
    }
    intermed_in
}

pub fn arrnum_to_str(
    src: &Vec<u8>,
    radix_def_dest : &RadixDef
        ) -> String {
    let mut str_out = String::new();
    for u in src.iter() {
        match radix_def_dest.from_u8(u.clone()) {
            Some(c) => {
                str_out.push(c);
            }
            None => {} //todo
        }
    }
    str_out
}

#[allow(unused_variables)]
pub fn base_conv_str(
    src: &str,
    radix_def_src : &RadixDef,
    radix_def_dest : &RadixDef
        ) -> String {
    let intermed_in : Vec<u8> =
        str_to_arrnum(src, radix_def_src);
    let intermed_out = base_conv_vec(
        &intermed_in,
        radix_def_src.get_max(),
        radix_def_dest.get_max(),
    );
    arrnum_to_str(&intermed_out, radix_def_dest)
}

pub trait RadixDef {
    fn get_max (&self) -> u8;
    fn from_char (&self, x:char) -> Option<u8>;
    fn from_u8 (&self, x:u8) -> Option<char>;
}
pub struct RadixTen;

const ZERO_ASC : u8 = '0' as u8;
const UPPER_A_ASC : u8 = 'A' as u8;
const LOWER_A_ASC : u8 = 'a' as u8;    

impl RadixDef for RadixTen {
    fn get_max(&self) -> u8 { 10 }
    fn from_char (&self, c:char) -> Option<u8> {
        match c {
            '0'...'9' => Some(c as u8 - ZERO_ASC),
            _ => None
        }
    }
    fn from_u8 (&self, u:u8) -> Option<char> {
        match u {
            0...9 => Some((ZERO_ASC + u) as char),
            _ => None
        }    
    }
}
pub struct RadixHex;
impl RadixDef for RadixHex {
    fn get_max(&self) -> u8 { 16 }
    fn from_char (&self, c:char) -> Option<u8> {
        match c {
            '0'...'9' => Some(c as u8 - ZERO_ASC),
            'A'...'F' => Some(c as u8 +10 - UPPER_A_ASC),
            'a'...'f' => Some(c as u8 +10 - LOWER_A_ASC),
            _ => None
        }
    }
    fn from_u8 (&self, u:u8) -> Option<char> {
        match u {
            0...9 => Some((ZERO_ASC + u) as char),
            10...15 => Some((UPPER_A_ASC + (u-10)) as char),
            _ => None
        }    
    }
}

