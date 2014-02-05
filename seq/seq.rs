#[crate_id(name="seq", vers="1.0.0", author="Daniel MacDougall")];

// TODO: Make -w flag work with decimals
// TODO: Support -f flag

extern mod extra;

use std::os;
use std::cmp::max;
use extra::getopts::groups;

fn print_usage(opts: ~[groups::OptGroup]) {
    println!("seq 1.0.0\n");
    println!("Usage:\n  seq [-w] [-s string] [-t string] [first [step]] last\n");
    println!("{:s}", groups::usage("Print sequences of numbers", opts));
}

fn parse_float(s: &str) -> Result<f32, ~str>{
    match from_str(s) {
        Some(n) => Ok(n),
        None => Err(format!("seq: invalid floating point argument: {:s}", s))
    }
}

fn escape_sequences(s: &str) -> ~str {
    s.replace("\\n", "\n").
        replace("\\t", "\t")
}

fn main() {
    let args = os::args();
    let opts = ~[
        groups::optopt("s", "separator", "Separator character (defaults to \\n)", ""),
        groups::optopt("t", "terminator", "Terminator character (defaults to separator)", ""),
        groups::optflag("w", "widths", "Equalize widths of all numbers by padding with zeros"),
        groups::optflag("h", "help", "print this help text and exit"),
        groups::optflag("V", "version", "print version and exit"),
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => {
            println!("{:s}", f.to_err_msg());
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
        print_usage(opts);
        return;
    }
    let first = if matches.free.len() > 1 {
        match parse_float(matches.free[0]) {
            Ok(n) => n,
            Err(s) => { println!("{:s}", s); return; }
        }
    } else {
        1.0
    };
    let step = if matches.free.len() > 2 {
        match parse_float(matches.free[1]) {
            Ok(n) => n,
            Err(s) => { println!("{:s}", s); return; }
        }
    } else {
        1.0
    };
    let last = match parse_float(matches.free[matches.free.len()-1]) {
        Ok(n) => n,
        Err(s) => { println!("{:s}", s); return; }
    };
    let separator = escape_sequences(matches.opt_str("s").unwrap_or(~"\n"));
    let terminator = escape_sequences(matches.opt_str("t").unwrap_or(separator.clone()));
    print_seq(first, step, last, separator, terminator, matches.opt_present("w"));
}

fn done_printing(next: f32, step: f32, last: f32) -> bool {
    if step > 0f32 {
        next > last
    } else {
        next < last
    }
}

fn print_seq(first: f32, step: f32, last: f32, separator: ~str, terminator: ~str, pad: bool) {
    let mut i = first;
    let maxlen = max(first, last).to_str().len();
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
