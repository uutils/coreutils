#![crate_name = "echo"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::io::Write;
use std::str::from_utf8;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[allow(dead_code)]
static NAME: &'static str = "echo";
static VERSION: &'static str = "1.0.0";

#[derive(Clone)]
struct EchoOptions {
    newline: bool,
    escape: bool
}

#[inline(always)]
fn to_char(bytes: &Vec<u8>, base: u32) -> char {
    usize::from_str_radix(from_utf8(bytes.as_ref()).unwrap(), base).unwrap() as u8 as char
}

#[inline(always)]
fn isxdigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' |
        '8' | '9' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' => true,
        _ => false
    }
}

#[inline(always)]
fn isodigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' => true,
        _ => false
    }
}

fn convert_str(string: &[u8], index: usize, base: u32) -> (char, usize) {
    let (max_digits, is_legal_digit) : (usize, fn(u8) -> bool) = match base {
        8 => (3, isodigit),
        16 => (2, isxdigit),
        _ => panic!(),
    };

    let mut bytes = vec!();
    for offset in (0usize .. max_digits) {
        if string.len() <= index + offset as usize {
            break;
        }
        let c = string[index + offset as usize];
        if is_legal_digit(c) {
            bytes.push(c as u8);
        } else {
            break;
        }
    }

    if bytes.len() == 0 {
        (' ', 0)
    } else {
        (to_char(&bytes, base), bytes.len())
    }
}

fn parse_options(args: Vec<String>, options: &mut EchoOptions) -> Option<Vec<String>> {
    let mut echo_args = vec!();
    'argloop: for arg in args.into_iter().skip(1) {
        match arg.as_ref() {
            "--help" | "-h" => {
                print_help();
                return None;
            }
            "--version" | "-V" => {
                print_version();
                return None;
            }
            "-n" => options.newline = true,
            "-e" => options.escape = true,
            "-E" => options.escape = false,
            _ => {
                if arg.len() > 1 && arg.chars().next().unwrap_or('_') == '-' {
                    let mut newopts = options.clone();
                    for ch in arg.chars().skip(1) {
                        match ch {
                            'h' => {
                                print_help();
                                return None;
                            }
                            'V' => {
                                print_version();
                                return None;
                            }
                            'n' => newopts.newline = true,
                            'e' => newopts.escape = true,
                            'E' => newopts.escape = false,
                            _ => {
                                echo_args.push(arg.clone());
                                continue 'argloop;
                            }
                        }
                    }
                    *options = newopts;
                } else {
                    echo_args.push(arg);
                }
            }
        }
    }
    Some(echo_args)
}

fn print_help() {
    let mut opts = getopts::Options::new();
    opts.optflag("n", "", "do not output the trailing newline");
    opts.optflag("e", "", "enable interpretation of backslash escapes");
    opts.optflag("E", "", "disable interpretation of backslash escapes (default)");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let msg = format!("{0} {1} - display a line of text

Usage:
  {0} [SHORT-OPTION]... [STRING]...
  {0} LONG-OPTION

Echo the STRING(s) to standard output.
If -e is in effect, the following sequences are recognized:

\\\\      backslash
\\a      alert (BEL)
\\b      backspace
\\c      produce no further output
\\e      escape
\\f      form feed
\\n      new line
\\r      carriage return
\\t      horizontal tab
\\v      vertical tab
\\0NNN   byte with octal value NNN (1 to 3 digits)
\\xHH    byte with hexadecimal value HH (1 to 2 digits)", NAME, VERSION);

    print!("{}", opts.usage(&msg));
}

fn print_version() {
    println!("{} {}", NAME, VERSION);
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut options = EchoOptions {
        newline: false,
        escape: false
    };

    let free = match parse_options(args, &mut options) {
        Some(vec) => vec,
        None => return 0
    };

    if !free.is_empty() {
        let string = free.join(" ");
        if options.escape {
            let mut prev_was_slash = false;
            let mut iter = string.chars().enumerate();
            loop {
                match iter.next() {
                    Some((index, c)) => {
                        if !prev_was_slash {
                            if c != '\\' {
                                print!("{}", c);
                            } else {
                                prev_was_slash = true;
                            }
                        } else {
                            prev_was_slash = false;
                            match c {
                                '\\' => print!("\\"),
                                'a' => print!("\x07"),
                                'b' => print!("\x08"),
                                'c' => break,
                                'e' => print!("\x1B"),
                                'f' => print!("\x0C"),
                                'n' => print!("\n"),
                                'r' => print!("\r"),
                                't' => print!("\t"),
                                'v' => print!("\x0B"),
                                'x' => {
                                    let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 16);
                                    if num_char_used == 0 {
                                        print!("\\x");
                                    } else {
                                        print!("{}", c);
                                        for _ in (0 .. num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                },
                                '0' => {
                                    let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 8);
                                    if num_char_used == 0 {
                                        print!("\0");
                                    } else {
                                        print!("{}", c);
                                        for _ in (0 .. num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                }
                                _ => {
                                    let (esc_c, num_char_used) = convert_str(string.as_bytes(), index, 8);
                                    if num_char_used == 0 {
                                        print!("\\{}", c);
                                    } else {
                                        print!("{}", esc_c);
                                        for _ in (1 .. num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => break
                }
            }
        } else {
            print!("{}", string);
        }
    }

    if options.newline {
        pipe_flush!();
    } else {
        println!("")
    }

    0
}
