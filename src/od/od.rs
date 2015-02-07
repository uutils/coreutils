#![crate_name = "od"]
#![feature(collections, core, io, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Hirsch <benhirsch24@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate collections;

use collections::string::String;
use std::old_io::File;

#[derive(Debug)]
enum Radix { Decimal, Hexadecimal, Octal, Binary }

pub fn uumain(args: Vec<String>) -> i32 {
    let opts = [
        getopts::optopt("A", "address-radix",
                        "Select the base in which file offsets are printed.", "RADIX"),
        getopts::optopt("j", "skip-bytes",
                        "Skip bytes input bytes before formatting and writing.", "BYTES"),
        getopts::optopt("N", "read-bytes",
                        "limit dump to BYTES input bytes", "BYTES"),
        getopts::optopt("S", "strings",
                        ("output strings of at least BYTES graphic chars. 3 is assumed when \
                          BYTES is not specified."),
                        "BYTES"),
        getopts::optopt("t", "format", "select output format or formats", "TYPE"),
        getopts::optflag("v", "output-duplicates", "do not use * to mark line suppression"),
        getopts::optopt("w", "width",
                        ("output BYTES bytes per output line. 32 is implied when BYTES is not \
                          specified."),
                        "BYTES"),
        getopts::optflag("h", "help", "display this help and exit."),
        getopts::optflag("", "version", "output version information and exit."),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f)
    };

    let input_offset_base = match parse_radix(matches.opt_str("A")) {
        Ok(r) => r,
        Err(f) => { panic!("Invalid -A/--address-radix\n{}", f) }
    };

    let fname = match args.last() {
        Some(n) => n,
        None => { panic!("Need fname for now") ; }
    };

    main(&input_offset_base, fname.as_slice());

    0
}

fn main(input_offset_base: &Radix, fname: &str) {
    let mut f = match File::open(&Path::new(fname)) {
        Ok(f) => f,
        Err(e) => panic!("file error: {}", e)
    };

    let mut addr = 0;
    let bytes = &mut [b'\x00'; 16];
    loop {
        match f.read(bytes) {
            Ok(n) => {
                print_with_radix(input_offset_base, addr);
                for b in range(0, n / std::u16::BYTES) {
                    let bs = &bytes[(2 * b) .. (2 * b + 2)];
                    let p: u16 = (bs[1] as u16) << 8 | bs[0] as u16;
                    print!(" {:06o}", p);
                }
                if n % std::u16::BYTES == 1 {
                    print!(" {:06o}", bytes[n - 1]);
                }
                print!("\n");
                addr += n;
            },
            Err(_) => {
                print_with_radix(input_offset_base, addr);
                break;
            }
        };
    };
}

fn parse_radix(radix_str: Option<String>) -> Result<Radix, &'static str> {
    match radix_str {
        None => Ok(Radix::Octal),
        Some(s) => {
            let st = s.into_bytes();
            if st.len() != 1 {
                Err("Radix must be one of [d, o, b, x]\n")
            } else {
                let radix: char = *(st.get(0)
                                      .expect("byte string of length 1 lacks a 0th elem")) as char;
                match radix {
                    'd' => Ok(Radix::Decimal),
                    'x' => Ok(Radix::Hexadecimal),
                    'o' => Ok(Radix::Octal),
                    'b' => Ok(Radix::Binary),
                    _ => Err("Radix must be one of [d, o, b, x]\n")
                }
            }
        }
    }
}

fn print_with_radix(r: &Radix, x: usize) {
    // TODO(keunwoo): field widths should be based on sizeof(x), or chosen dynamically based on the
    // expected range of address values.  Binary in particular is not great here.
    match *r {
        Radix::Decimal => print!("{:07}", x),
        Radix::Hexadecimal => print!("{:07X}", x),
        Radix::Octal => print!("{:07o}", x),
        Radix::Binary => print!("{:07b}", x)
    }
}
