#![crate_name = "echo"]
#![feature(collections, core, io, libc, rustc_private)]

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

use std::old_io::{print, println};
use std::num::from_str_radix;
use std::str::from_utf8;

#[path = "../common/util.rs"]
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
fn to_char(bytes: &Vec<u8>, base: usize) -> char {
    from_str_radix::<usize>(from_utf8(bytes.as_slice()).unwrap(), base).unwrap() as u8 as char
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

fn convert_str(string: &[u8], index: usize, base: usize) -> (char, usize) {
    let (max_digits, is_legal_digit) : (usize, fn(u8) -> bool) = match base {
        8us => (3, isodigit),
        16us => (2, isxdigit),
        _ => panic!(),
    };

    let mut bytes = vec!();
    for offset in range(0us, max_digits) {
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
    let program = args[0].clone();
    'argloop: for arg in args.into_iter().skip(1) {
        match arg.as_slice() {
            "--help" | "-h" => {
                print_help(&program);
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
                if arg.len() > 1 && arg.as_slice().char_at(0) == '-' {
                    let mut newopts = options.clone();
                    let argptr: *const String = &arg;  // escape from the borrow checker
                    for ch in unsafe { (*argptr).as_slice() }.chars().skip(1) {
                        match ch {
                            'h' => {
                                print_help(&program);
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
                                echo_args.push(arg);
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

fn print_help(program: &String) {
    let opts = [
        getopts::optflag("n", "", "do not output the trailing newline"),
        getopts::optflag("e", "", "enable interpretation of backslash escapes"),
        getopts::optflag("E", "", "disable interpretation of backslash escapes (default)"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];
    println!("echo {} - display a line of text", VERSION);
    println!("");
    println!("Usage:");
    println!("  {0} [SHORT-OPTION]... [STRING]...", *program);
    println!("  {0} LONG-OPTION", *program);
    println!("");
    println(getopts::usage("Echo the STRING(s) to standard output.", &opts).as_slice());
    println("If -e is in effect, the following sequences are recognized:

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
\\xHH    byte with hexadecimal value HH (1 to 2 digits)");
}

fn print_version() {
    println!("echo version: {}", VERSION);
}

pub fn uumain(args: Vec<String>) -> isize {
    let mut options = EchoOptions {
        newline: false,
        escape: false
    };

    let free = match parse_options(args, &mut options) {
        Some(vec) => vec,
        None => return 0
    };

    if !free.is_empty() {
        let string = free.connect(" ");
        if options.escape {
            let mut prev_was_slash = false;
            let mut iter = string.as_slice().chars().enumerate();
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
                                    let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 16us);
                                    if num_char_used == 0 {
                                        print!("\\x");
                                    } else {
                                        print!("{}", c);
                                        for _ in range(0, num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                },
                                '0' => {
                                    let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 8us);
                                    if num_char_used == 0 {
                                        print!("\0");
                                    } else {
                                        print!("{}", c);
                                        for _ in range(0, num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                }
                                _ => {
                                    let (esc_c, num_char_used) = convert_str(string.as_bytes(), index, 8us);
                                    if num_char_used == 0 {
                                        print!("\\{}", c);
                                    } else {
                                        print!("{}", esc_c);
                                        for _ in range(1, num_char_used) {
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
            print(string.as_slice());
        }
    }

    if !options.newline {
        println!("")
    }

    0
}
