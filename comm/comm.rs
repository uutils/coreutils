#![crate_id(name="comm", vers="1.0.0", author="Michael Gehring")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::cmp::TotalOrd;
use std::io::{BufferedReader, IoResult, print};
use std::io::fs::File;
use std::io::stdio::stdin;
use std::os;
use std::path::Path;

static NAME : &'static str = "comm";
static VERSION : &'static str = "1.0.0";

fn mkdelim(col: uint, opts: &getopts::Matches) -> String {
    let mut s = String::new();
    let delim = match opts.opt_str("output-delimiter") {
        Some(d) => d.clone(),
        None => "\t".to_string(),
    };

    if col > 1 && !opts.opt_present("1") {
        s.push_str(delim.as_slice());
    }
    if col > 2 && !opts.opt_present("2") {
        s.push_str(delim.as_slice());
    }

    s
}

fn comm(a: &mut Box<Buffer>, b: &mut Box<Buffer>, opts: &getopts::Matches) {

    let delim = Vec::from_fn(4, |col| mkdelim(col, opts));

    let mut ra = a.read_line();
    let mut rb = b.read_line();

    while ra.is_ok() || rb.is_ok() {
        let ord = match (ra.clone(), rb.clone()) {
            (Err(_), Ok(_))  => Greater,
            (Ok(_) , Err(_)) => Less,
            (Ok(s0), Ok(s1)) => s0.cmp(&s1),
            _ => unreachable!(),
        };

        match ord {
            Less => {
                if !opts.opt_present("1") {
                    print!("{}{}", delim.get(1), ra.unwrap());
                }
                ra = a.read_line();
            }
            Greater => {
                if !opts.opt_present("2") {
                    print!("{}{}", delim.get(2), rb.unwrap());
                }
                rb = b.read_line();
            }
            Equal => {
                if !opts.opt_present("3") {
                    print!("{}{}", delim.get(3), ra.unwrap());
                }
                ra = a.read_line();
                rb = b.read_line();
            }
        }
    }
}

fn open_file(name: &str) -> IoResult<Box<Buffer>> {
    match name {
        "-" => Ok(box stdin() as Box<Buffer>),
        _   => {
            let f = try!(File::open(&Path::new(name)));
            Ok(box BufferedReader::new(f) as Box<Buffer>)
        }
    }
}

pub fn main() {
    let args = os::args();
    let opts = [
        getopts::optflag("1", "", "suppress column 1 (lines uniq to FILE1)"),
        getopts::optflag("2", "", "suppress column 2 (lines uniq to FILE2)"),
        getopts::optflag("3", "", "suppress column 3 (lines that appear in both files)"),
        getopts::optopt("", "output-delimiter", "separate columns with STR", "STR"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(err) => fail!("{}", err.to_err_msg()),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return;
    }

    if matches.opt_present("help") || matches.free.len() != 2 {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] FILE1 FILE2", NAME);
        println!("");
        print(getopts::usage("Compare sorted files line by line.", opts.as_slice()).as_slice());
        if matches.free.len() != 2 {
            os::set_exit_status(1);
        }
        return;
    }


    let mut f1 = open_file(matches.free.get(0).as_slice()).unwrap();
    let mut f2 = open_file(matches.free.get(1).as_slice()).unwrap();

    comm(&mut f1, &mut f2, &matches)
}
