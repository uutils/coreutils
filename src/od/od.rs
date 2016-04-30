#![crate_name = "uu_od"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Hirsch <benhirsch24@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::fs::File;
use std::io::Read;
use std::mem;
use std::io::BufReader;
use std::io::Write;
use std::io;
 
#[derive(Debug)]
enum Radix { Decimal, Hexadecimal, Octal, Binary }
 
#[derive(Debug)]
enum InputSource<'a> {
    FileName(&'a str ),
    Stdin
}
 
pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("A", "address-radix",
                "Select the base in which file offsets are printed.", "RADIX");
    opts.optopt("j", "skip-bytes",
                "Skip bytes input bytes before formatting and writing.", "BYTES");
    opts.optopt("N", "read-bytes",
                "limit dump to BYTES input bytes", "BYTES");
    opts.optopt("S", "strings",
                ("output strings of at least BYTES graphic chars. 3 is assumed when \
                 BYTES is not specified."),
                "BYTES");
    opts.optopt("t", "format", "select output format or formats", "TYPE");
    opts.optflag("v", "output-duplicates", "do not use * to mark line suppression");
    opts.optopt("w", "width",
                ("output BYTES bytes per output line. 32 is implied when BYTES is not \
                 specified."),
                "BYTES");
    opts.optflag("h", "help", "display this help and exit.");
    opts.optflag("", "version", "output version information and exit.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f)
    };

    let input_offset_base = match parse_radix(matches.opt_str("A")) {
        Ok(r) => r,
        Err(f) => { panic!("Invalid -A/--address-radix\n{}", f) }
    };
 
    // Gather up file names - args which don't start with '-'
    let fnames = args[1..]
                     .iter()
                     .filter(|w| !w.starts_with('-') || w == &"--" ) // "--" starts with '-', but it denotes stdin, not a flag 
                     .map(|x| match x.as_str() { "--" => InputSource::Stdin, x => InputSource::FileName(x)})
                     .collect::<Vec<_>>();
 
    // With no filenames, od uses stdin as input.
    if fnames.len() == 0 {
        odfunc(&input_offset_base, &[InputSource::Stdin])
    }
    else {
        odfunc(&input_offset_base, &fnames)
    }
}

const LINEBYTES:usize = 16;
const WORDBYTES:usize = 2;
 
fn odfunc(input_offset_base: &Radix, fnames: &[InputSource]) -> i32 {
 
    let mut status = 0;
    let mut ni = fnames.iter();
    {
        // Open and return the next file to process as a BufReader
        // Returns None when no more files.
        let mut next_file = || -> Option<Box<io::Read>> {
            // loop retries with subsequent files if err - normally 'loops' once
            loop {
                match ni.next() {
                    None => return None,
                    Some(input) => match *input {
                        InputSource::Stdin => return Some(Box::new(BufReader::new(std::io::stdin()))),
                        InputSource::FileName(fname) => match File::open(fname) {
                            Ok(f) => return Some(Box::new(BufReader::new(f))),
                            Err(e) => {
                                // If any file can't be opened,
                                // print an error at the time that the file is needed,
                                // then move on the the next file.
                                // This matches the behavior of the original `od`
                                let _ = writeln!(&mut std::io::stderr(), "od: '{}': {}", fname, e);
                                if status == 0 {status = 1}
                            }
                        }
                    }
                }
            }
        };
 
        let mut curr_file: Box<io::Read> = match next_file() {
            Some(f) => f, 
            None => {
                return 1;
            } 
        };
 
        let mut exhausted = false; // There is no more input, gone to the end of the last file.

        // Fill buf with bytes read from the list of files
        // Returns Ok(<number of bytes read>) 
        // Handles io errors itself, thus always returns OK
        // Fills the provided buffer completely, unless it has run out of input.
        // If any call returns short (< buf.len()), all subsequent calls will return Ok<0>
        let mut f_read = |buf: &mut [u8]| -> io::Result<usize> {
            if exhausted {
                Ok(0)
            } else {
                let mut xfrd = 0;
                // while buffer we are filling is not full.. May go thru several files.
                'fillloop: while xfrd < buf.len() {
                    loop { // stdin may return on 'return' (enter), even though the buffer isn't full.
                        xfrd += match curr_file.read(&mut buf[xfrd..]) {
                            Ok(0) => break, 
                            Ok(n) => n,
                            Err(e) => panic!("file error: {}", e),
                        };
                        if xfrd == buf.len() {
                            // transferred all that was asked for.
                            break 'fillloop;
                        }
                    }
                    curr_file = match next_file() { 
                        Some(f) => f, 
                        None => {
                            exhausted = true;
                            break;
                        } 
                    };
                }
                Ok(xfrd)
            }
        };
 
        let mut addr = 0;
        let bytes = &mut [b'\x00'; LINEBYTES];
        loop { // print each line
            print_with_radix(input_offset_base, addr); // print offset
            match f_read(bytes) {
                Ok(0) => {
                    print!("\n");
                    break;
                }
                Ok(n) => {
                    print!("  "); // 4 spaces after offset - we print 2 more before each word
                 
                    for b in 0 .. n / mem::size_of::<u16>() {
                        let bs = &bytes[(2 * b) .. (2 * b + 2)];
                        let p: u16 = (bs[1] as u16) << 8 | bs[0] as u16;
                        print!("  {:06o}", p);
                    }
                    if n % mem::size_of::<u16>() == 1 {
                        print!("  {:06o}", bytes[n - 1]);
                    }
 
                    // Add extra spaces to pad out the short, presumably last, line.
                    if n<LINEBYTES {
                        // calc # of items we did not print, must be short at least WORDBYTES to be missing any.
                        let words_short = (LINEBYTES-n)/WORDBYTES; 
                        print!("{:>width$}", "", width=(words_short)*(6+2));
                    }
 
                    print!("\n");
                    addr += n;
                },
                Err(_) => {
                    break;
                }
            };
        };
    };
    status
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
