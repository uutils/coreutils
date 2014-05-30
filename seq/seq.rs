#![crate_id(name="seq", vers="1.0.0", author="Daniel MacDougall")]

#![feature(macro_rules)]

// TODO: Make -w flag work with decimals
// TODO: Support -f flag

extern crate getopts;
extern crate libc;

use std::os;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "seq";

fn print_usage(opts: &[getopts::OptGroup]) {
    println!("seq 1.0.0\n");
    println!("Usage:\n  seq [-w] [-s string] [-t string] [first [step]] last\n");
    println!("{:s}", getopts::usage("Print sequences of numbers", opts));
}

fn parse_float(s: &str) -> Result<f32, String>{
    match from_str(s) {
        Some(n) => Ok(n),
        None => Err(format!("seq: invalid floating point argument: {:s}", s))
    }
}

fn escape_sequences(s: &str) -> String {
    s.replace("\\n", "\n").
        replace("\\t", "\t")
}

#[allow(dead_code)]
fn main() { uumain(os::args()); }

pub fn uumain(args: Vec<String>) {
    let opts = [
        getopts::optopt("s", "separator", "Separator character (defaults to \\n)", ""),
        getopts::optopt("t", "terminator", "Terminator character (defaults to separator)", ""),
        getopts::optflag("w", "widths", "Equalize widths of all numbers by padding with zeros"),
        getopts::optflag("h", "help", "print this help text and exit"),
        getopts::optflag("V", "version", "print version and exit"),
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => {
            show_error!(1, "{:s}", f.to_err_msg());
            print_usage(opts);
            return;
        }
    };
    if matches.opt_present("help") {
        print_usage(opts);
        return;
    }
    if matches.opt_present("version") {
        println!("seq 1.0.0");
        return;
    }
    if matches.free.len() < 1 || matches.free.len() > 3 {
        os::set_exit_status(1);
        print_usage(opts);
        return;
    }
    let first = if matches.free.len() > 1 {
        match parse_float(matches.free.get(0).as_slice()) {
            Ok(n) => n,
            Err(s) => { show_error!(1, "{:s}", s); return; }
        }
    } else {
        1.0
    };
    let step = if matches.free.len() > 2 {
        match parse_float(matches.free.get(1).as_slice()) {
            Ok(n) => n,
            Err(s) => { show_error!(1, "{:s}", s); return; }
        }
    } else {
        1.0
    };
    let last = match parse_float(matches.free.get(matches.free.len()-1).as_slice()) {
        Ok(n) => n,
        Err(s) => { show_error!(1, "{:s}", s); return; }
    };
    let separator = escape_sequences(matches.opt_str("s").unwrap_or("\n".to_string()).as_slice());
    let terminator = escape_sequences(matches.opt_str("t").unwrap_or(separator.to_string()).as_slice());
    print_seq(first, step, last, separator, terminator, matches.opt_present("w"));
}

fn done_printing(next: f32, step: f32, last: f32) -> bool {
    if step > 0f32 {
        next > last
    } else {
        next < last
    }
}

fn print_seq(first: f32, step: f32, last: f32, separator: String, terminator: String, pad: bool) {
    let mut i = first;
    let maxlen = first.max(last).to_str().len();
    while !done_printing(i, step, last) {
        let ilen = i.to_str().len();
        if pad && ilen < maxlen {
            for _ in range(0, maxlen - ilen) {
                print!("0");
            }
        }
        print!("{:f}", i);
        i += step;
        if !done_printing(i, step, last) {
            print!("{:s}", separator);
        }
    }
    print!("{:s}", terminator);
}
