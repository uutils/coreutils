#[crate_id(name="echo", vers="1.0.0", author="Derek Chiang")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{print, println, stderr};
use std::uint;
use extra::getopts::groups;

static VERSION: &'static str = "1.0.0";

fn print_char(c: char) {
    print!("{}", c);
}

fn to_char(bytes: &[u8], base: uint) -> char {
    uint::parse_bytes(bytes, base).unwrap() as u8 as char
}

fn isxdigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' |
        '8' | '9' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' => true,
        _ => false
    }
}

fn isodigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' => true,
        _ => false
    }
}

fn convert_str(string: &str, index: uint, base: uint) -> (char, int) {
    let (max_digits, is_legal_digit) = match base {
        8u => (3, isodigit),
        16u => (2, isxdigit),
        _ => fail!(),
    };

    let mut bytes: ~[u8] = ~[];
    for offset in range(0, max_digits) {
        let c = string[index + offset as uint];
        if is_legal_digit(c) {
            bytes.push(c as u8);
        } else {
            if bytes.len() > 0 {
                return (to_char(bytes, base), offset);
            } else {
                return (' ', offset);
            }
        }
    }
    return (to_char(bytes, base), max_digits)
}

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("n", "", "do not output the trailing newline"),
        groups::optflag("e", "", "enable interpretation of backslash escapes"),
        groups::optflag("E", "", "disable interpretation of backslash escapes (default)"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }  
    };

    if matches.opt_present("help") {
        println!("echo {:s} - display a line of text", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [SHORT-OPTION]... [STRING]...", program);
        println!("  {0:s} LONG-OPTION", program);
        println!("");
        println(groups::usage("Echo the STRING(s) to standard output.", opts));
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
        return;
    }

    if matches.opt_present("version") {
        return println!("echo version: {:s}", VERSION);
    }

    if !matches.free.is_empty() {
        let string = matches.free.connect(" ");
        if matches.opt_present("e") {
            let mut prev_was_slash = false;
            let mut iter = string.chars().enumerate();
            loop {
                match iter.next() {
                    Some((index, c)) => {
                        if !prev_was_slash {
                            if c != '\\' {
                                print_char(c);
                            } else {
                                prev_was_slash = true;
                            }
                        } else {
                            prev_was_slash = false;
                            match c {
                                '\\' => print_char('\\'),
                                'a' => print_char('\x07'),
                                'b' => print_char('\x08'),
                                'c' => break,
                                'e' => print_char('\x1B'),
                                'f' => print_char('\x0C'),
                                'n' => print_char('\n'),
                                'r' => print_char('\r'),
                                't' => print_char('\t'),
                                'v' => print_char('\x0B'),
                                'x' => {
                                    let (c, num_char_used) = convert_str(string, index + 1, 16u);
                                    if num_char_used == 0 {
                                        print_char('\\');
                                        print_char('x');
                                    } else {
                                        print_char(c);
                                        for _ in range(0, num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                },
                                '0' => {
                                    let (c, num_char_used) = convert_str(string, index + 1, 8u);
                                    if num_char_used == 0 {
                                        print_char('\\');
                                        print_char('0');
                                    } else {
                                        print_char(c);
                                        for _ in range(0, num_char_used) {
                                            iter.next(); // consume used characters
                                        }
                                    }
                                }
                                _ => {
                                    print_char('\\');
                                    print_char(c);
                                }
                            }
                        }
                    }
                    None => break
                }
            }
        } else {
            print(string);
        }
    }

    if !matches.opt_present("n") {
        println!("")
    }
}
