#[link(name="echo", vers="1.0.0", author="Derek Chiang")];

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
use std::io::stderr;
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
        println("echo " + VERSION + " - display a line of text");
        println("");
        println("Usage:");
        println!("  {0:s} [SHORT-OPTION]... [STRING]...", program);
        println!("  {0:s} LONG-OPTION", program);
        println("");
        print(groups::usage("Echo the STRING(s) to standard output.", opts));
        print("\nIf -e is in effect, the following sequences are recognized:\n\
\n\
  \\\\      backslash\n\
  \\a      alert (BEL)\n\
  \\b      backspace\n\
  \\c      produce no further output\n\
  \\e      escape\n\
  \\f      form feed\n\
  \\n      new line\n\
  \\r      carriage return\n\
  \\t      horizontal tab\n\
  \\v      vertical tab\n\
  \\0NNN   byte with octal value NNN (1 to 3 digits)\n\
  \\xHH    byte with hexadecimal value HH (1 to 2 digits)\n\
");
        return;
    }

    if matches.opt_present("version") {
        return println("echo version: " + VERSION);
    }

    if matches.free.is_empty() {
        print("");
    } else {
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
                                    if index == string.len() - 1 {
                                        print_char('\\');
                                        print_char('x');
                                    } else if index == string.len() - 2 {
                                        let next_char = string[index + 1];
                                        if isxdigit(next_char) {
                                            print_char(to_char([next_char as u8], 16u));
                                            iter.next();
                                        } else {
                                            print_char('\\');
                                            print_char('x');
                                        }
                                    } else {
                                        let next_char = string[index + 1];
                                        let next_next_char = string[index + 2];
                                        match (isxdigit(next_char), isxdigit(next_next_char)) {
                                            (true, true) => {
                                                print_char(to_char([next_char as u8, next_next_char as u8], 16u));
                                                iter.next(); iter.next();
                                            }
                                            (true, false) => {
                                                print_char(to_char([next_char as u8], 16u));
                                                iter.next();
                                            }
                                            _ => {
                                                print_char('\\');
                                                print_char('x');
                                            }
                                        };
                                    }
                                },
                                '0' => {
                                    if index == string.len() - 1 {
                                        print_char('\\');
                                        print_char('0');
                                    } else if index == string.len() - 2 {
                                        let next_char = string[index + 1];
                                        if isodigit(next_char) {
                                            print_char(to_char([next_char as u8], 8u));
                                            iter.next();
                                        } else {
                                            print_char('\\');
                                            print_char('0');
                                        }
                                    } else if index == string.len() - 3 {
                                        let next_char = string[index + 1];
                                        let next_next_char = string[index + 2];
                                        match (isodigit(next_char), isodigit(next_next_char)) {
                                            (true, true) => {
                                                print_char(to_char([next_char as u8, next_next_char as u8], 8u));
                                                iter.next(); iter.next();
                                            }
                                            (true, false) => {
                                                print_char(to_char([next_char as u8], 8u));
                                                iter.next();
                                            }
                                            _ => {
                                                print_char('\\');
                                                print_char('x');
                                            }
                                        };
                                    } else {
                                        let next_char = string[index + 1];
                                        let next_next_char = string[index + 2];
                                        let next_next_next_char = string[index + 3];
                                        match (isodigit(next_char), isodigit(next_next_char),
                                            isodigit(next_next_next_char)) {
                                            (true, true, true) => {
                                                print_char(to_char([next_char as u8, next_next_char as u8, next_next_next_char as u8], 8u));
                                                iter.next(); iter.next(); iter.next();
                                            }
                                            (true, true, false) => {
                                                print_char(to_char([next_char as u8, next_next_char as u8], 8u));
                                                iter.next(); iter.next();
                                            }
                                            (true, false, false) => {
                                                print_char(to_char([next_char as u8], 8u));
                                                iter.next();
                                            }
                                            _ => {
                                                print_char('\\');
                                                print_char('x');
                                            }
                                        };
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
        println("")
    }
}
