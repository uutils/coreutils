#![crate_id(name="seq", vers="1.0.0", author="Daniel MacDougall")]

#![feature(macro_rules)]

// TODO: Make -w flag work with decimals
// TODO: Support -f flag

extern crate getopts;
extern crate libc;

use std::cmp;
use std::io;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "seq";

macro_rules! pipe_write(
    ($($args:expr),+) => (
        match write!(&mut io::stdout() as &mut Writer, $($args),+) {
            Ok(_) => {}
            Err(f) =>
                if f.kind == io::BrokenPipe {
                    return
                } else {
                    fail!("{}", f.to_str())
                }
        }
    )
)

#[deriving(Clone)]
struct SeqOptions {
    separator: String,
    terminator: Option<String>,
    widths: bool
}

fn parse_float(s: &str) -> Result<f64, String>{
    match from_str(s) {
        Some(n) => Ok(n),
        None => Err(format!("seq: invalid floating point argument: {:s}", s))
    }
}

fn escape_sequences(s: &str) -> String {
    s.replace("\\n", "\n").
        replace("\\t", "\t")
}

fn parse_options(args: Vec<String>, options: &mut SeqOptions) -> Result<Vec<String>, int> {
    let mut seq_args = vec!();
    let program = args.get(0).clone();
    let mut iter = args.move_iter().skip(1);
    loop {
        match iter.next() {
            Some(arg) => match arg.as_slice() {
                "--help" | "-h" => {
                    print_help(&program);
                    return Err(0);
                }
                "--version" | "-V" => {
                    print_version();
                    return Err(0);
                }
                "-s" | "--separator" => match iter.next() {
                    Some(sep) => options.separator = sep,
                    None => {
                        show_error!("expected a separator after {}", arg);
                        return Err(1);
                    }
                },
                "-t" | "--terminator" => match iter.next() {
                    Some(term) => options.terminator = Some(term),
                    None => {
                        show_error!("expected a terminator after '{}'", arg);
                        return Err(1);
                    }
                },
                "-w" | "--widths" => options.widths = true,
                "--" => {
                    seq_args.push_all_move(iter.collect());
                    break;
                },
                _ => {
                    if arg.len() > 1 && arg.as_slice().char_at(0) == '-' {
                        let argptr: *const String = &arg;  // escape from the borrow checker
                        let mut chiter = unsafe { (*argptr).as_slice() }.chars().skip(1);
                        let mut ch = ' ';
                        while match chiter.next() { Some(m) => { ch = m; true } None => false } {
                            match ch {
                                'h' => {
                                    print_help(&program);
                                    return Err(0);
                                }
                                'V' => {
                                    print_version();
                                    return Err(0);
                                }
                                's' => match iter.next() {
                                    Some(sep) => {
                                        options.separator = sep;
                                        let next = chiter.next();
                                        if next.is_some() {
                                            show_error!("unexpected character ('{}')", next.unwrap());
                                            return Err(1);
                                        }
                                    }
                                    None => {
                                        show_error!("expected a separator after {}", arg);
                                        return Err(1);
                                    }
                                },
                                't' => match iter.next() {
                                    Some(term) => {
                                        options.terminator = Some(term);
                                        let next = chiter.next();
                                        if next.is_some() {
                                            show_error!("unexpected character ('{}')", next.unwrap());
                                            return Err(1);
                                        }
                                    }
                                    None => {
                                        show_error!("expected a terminator after {}", arg);
                                        return Err(1);
                                    }
                                },
                                'w' => options.widths = true,
                                _ => { seq_args.push(arg); break }
                            }
                        }
                    } else {
                        seq_args.push(arg);
                    }
                }
            },
            None => break
        }
    }
    Ok(seq_args)
}

fn print_help(program: &String) {
    let opts = [
        getopts::optopt("s", "separator", "Separator character (defaults to \\n)", ""),
        getopts::optopt("t", "terminator", "Terminator character (defaults to separator)", ""),
        getopts::optflag("w", "widths", "Equalize widths of all numbers by padding with zeros"),
        getopts::optflag("h", "help", "print this help text and exit"),
        getopts::optflag("V", "version", "print version and exit"),
    ];
    println!("seq 1.0.0\n");
    println!("Usage:\n  {} [-w] [-s string] [-t string] [first [step]] last\n", *program);
    println!("{:s}", getopts::usage("Print sequences of numbers", opts));
}

fn print_version() {
    println!("seq 1.0.0\n");
}

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0).clone();
    let mut options = SeqOptions {
        separator: "\n".to_string(),
        terminator: None,
        widths: false
    };
    let free = match parse_options(args, &mut options) {
        Ok(m) => m,
        Err(f) => return f
    };
    if free.len() < 1 || free.len() > 3 {
        crash!(1, "too {} operands.\nTry '{} --help' for more information.",
               if free.len() < 1 { "few" } else { "many" }, program);
    }
    let mut largest_dec = 0;
    let mut padding = 0;
    let first = if free.len() > 1 {
        let slice = free.get(0).as_slice();
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = len - dec;
        padding = dec;
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => { show_error!("{:s}", s); return 1; }
        }
    } else {
        1.0
    };
    let step = if free.len() > 2 {
        let slice = free.get(1).as_slice();
        let len = slice.len();
        let dec = slice.find('.').unwrap_or(len);
        largest_dec = cmp::max(largest_dec, len - dec);
        padding = cmp::max(padding, dec);
        match parse_float(free.get(1).as_slice()) {
            Ok(n) => n,
            Err(s) => { show_error!("{:s}", s); return 1; }
        }
    } else {
        1.0
    };
    let last = {
        let slice = free.get(free.len() - 1).as_slice();
        padding = cmp::max(padding, slice.find('.').unwrap_or(slice.len()));
        match parse_float(slice) {
            Ok(n) => n,
            Err(s) => { show_error!("{:s}", s); return 1; }
        }
    };
    let separator = escape_sequences(options.separator.as_slice());
    let terminator = match options.terminator {
        Some(term) => escape_sequences(term.as_slice()),
        None => separator.clone()
    };
    print_seq(first, step, last, largest_dec, separator, terminator, options.widths, padding);

    0
}

#[inline(always)]
fn done_printing(next: f64, step: f64, last: f64) -> bool {
    if step >= 0f64 {
        next > last
    } else {
        next < last
    }
}

fn print_seq(first: f64, step: f64, last: f64, largest_dec: uint, separator: String, terminator: String, pad: bool, padding: uint) {
    let mut i = 0i;
    let mut value = first + i as f64 * step;
    while !done_printing(value, step, last) {
        let istr = value.to_str();
        let ilen = istr.len();
        let before_dec = istr.as_slice().find('.').unwrap_or(ilen);
        if pad && before_dec < padding {
            for _ in range(0, padding - before_dec) {
                pipe_write!("0");
            }
        }
        pipe_write!("{}", istr);
        let mut idec = ilen - before_dec;
        if idec < largest_dec {
            if idec == 0 {
                pipe_write!(".");
                idec += 1;
            }
            for _ in range(idec, largest_dec) {
                pipe_write!("0")
            }
        }
        i += 1;
        value = first + i as f64 * step;
        if !done_printing(value, step, last) {
            pipe_write!("{:s}", separator);
        }
    }
    if (first >= last && step < 0f64) || (first <= last && step > 0f64) {
        pipe_write!("{:s}", terminator);
    }
}
