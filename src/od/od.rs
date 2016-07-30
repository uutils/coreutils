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
extern crate unindent;
extern crate byteorder;

#[macro_use]
extern crate uucore;

mod multifilereader;
mod byteorder_io;
mod formatteriteminfo;
mod prn_int;
mod prn_char;
mod prn_float;

use std::cmp;
use std::io::Write;
use unindent::*;
use byteorder_io::*;
use multifilereader::*;
use prn_int::*;
use prn_char::*;
use prn_float::*;
use formatteriteminfo::*;

//This is available in some versions of std, but not all that we target.
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
enum Radix { Decimal, Hexadecimal, Octal, Binary }

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("A", "address-radix",
                "Select the base in which file offsets are printed.", "RADIX");
    opts.optopt("j", "skip-bytes",
                "Skip bytes input bytes before formatting and writing.", "BYTES");
    opts.optopt("N", "read-bytes",
                "limit dump to BYTES input bytes", "BYTES");
    opts.optopt("", "endian", "byte order to use for multi-byte formats", "big|little");
    opts.optopt("S", "strings",
                ("output strings of at least BYTES graphic chars. 3 is assumed when \
                 BYTES is not specified."),
                "BYTES");
    opts.optflag("a", "", "named characters, ignoring high-order bit");
    opts.optflag("b", "", "octal bytes");
    opts.optflag("c", "", "ASCII characters or backslash escapes");
    opts.optflag("d", "", "unsigned decimal 2-byte units");
    opts.optflag("o", "", "unsigned decimal 2-byte units");

    opts.optflag("I", "", "decimal 2-byte units");
    opts.optflag("L", "", "decimal 2-byte units");
    opts.optflag("i", "", "decimal 2-byte units");
    opts.optflag("x", "", "hexadecimal 2-byte units");
    opts.optflag("h", "", "hexadecimal 2-byte units");

    opts.optflag("O", "", "octal 4-byte units");
    opts.optflag("s", "", "decimal 4-byte units");
    opts.optflag("X", "", "hexadecimal 4-byte units");
    opts.optflag("H", "", "hexadecimal 4-byte units");

    opts.optflag("e", "", "floating point double precision (64-bit) units");
    opts.optflag("f", "", "floating point single precision (32-bit) units");
    opts.optflag("F", "", "floating point double precision (64-bit) units");

    opts.optopt("t", "format", "select output format or formats", "TYPE");
    opts.optflag("v", "output-duplicates", "do not use * to mark line suppression");
    opts.optflagopt("w", "width",
                ("output BYTES bytes per output line. 32 is implied when BYTES is not \
                 specified."),
                "BYTES");
    opts.optflag("h", "help", "display this help and exit.");
    opts.optflag("", "version", "output version information and exit.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            disp_err!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("h") {
        let msg = unindent(&format!("
                Usage:
                    {0} [OPTION]... [FILENAME]...

                Displays data in various human-readable formats.", executable!()));
        println!("{}", opts.usage(&msg));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", executable!(), VERSION);
        return 0;
    }

    let input_offset_base = match parse_radix(matches.opt_str("A")) {
        Ok(r) => r,
        Err(f) => {
            disp_err!("Invalid -A/--address-radix\n{}", f);
            return 1;
        }
    };

    let byte_order = match matches.opt_str("endian").as_ref().map(String::as_ref) {
        None => { ByteOrder::Native },
        Some("little") => { ByteOrder::Little },
        Some("big") => { ByteOrder::Big },
        Some(s) => {
            disp_err!("Invalid argument --endian={}", s);
            return 1;
        }
    };

    // Gather up file names - args which don't start with '-'
    let stdnionly = [InputSource::Stdin];
    let inputs = args[1..]
        .iter()
        .filter_map(|w| match w as &str {
            "--" => Some(InputSource::Stdin),
            o if o.starts_with("-") => None,
            x => Some(InputSource::FileName(x)),
        })
        .collect::<Vec<_>>();
    // If no input files named, use stdin.
    let inputs = if inputs.len() == 0 {
        &stdnionly[..]
    } else {
        &inputs[..]
    };
    // Gather up format flags, we don't use getopts becase we need keep them in order.
    let flags = args[1..]
        .iter()
        .filter_map(|w| match w as &str {
            "--" => None,
            o if o.starts_with("-") => Some(&o[1..]),
            _ => None,
        })
        .collect::<Vec<_>>();

// TODO: -t fmts
        let known_formats = hashmap![
            "a" => FORMAT_ITEM_A,
            "B" => FORMAT_ITEM_OCT16,
            "b" => FORMAT_ITEM_OCT8,
            "c" => FORMAT_ITEM_C,
            "D" => FORMAT_ITEM_DEC32U,
            "e" => FORMAT_ITEM_F64,
            "F" => FORMAT_ITEM_F64,
            "f" => FORMAT_ITEM_F32,
            "H" => FORMAT_ITEM_HEX32,
            "X" => FORMAT_ITEM_HEX32,
            "o" => FORMAT_ITEM_OCT16,
            "x" => FORMAT_ITEM_HEX16,
            "h" => FORMAT_ITEM_HEX16,

            "I" => FORMAT_ITEM_DEC16S,
            "L" => FORMAT_ITEM_DEC16S,
            "i" => FORMAT_ITEM_DEC16S,

            "O" => FORMAT_ITEM_OCT32,
            "s" => FORMAT_ITEM_DEC16U
        ];

        let mut formats = Vec::new();

        for flag in flags.iter() {
            match known_formats.get(flag) {
                None => {} // not every option is a format
                Some(r) => {
                    formats.push(*r)
                }
            }
        }

        if formats.is_empty() {
            formats.push(FORMAT_ITEM_OCT16); // 2 byte octal is the default
        }

        let mut line_bytes = match matches.opt_default("w", "32") {
            None => 16,
            Some(s) => {
                match s.parse::<usize>() {
                    Ok(i) => { i }
                    Err(_) => { 2 }
                }
            }
        };
        let min_bytes = formats.iter().fold(2, |max, next| cmp::max(max, next.byte_size));
        if line_bytes % min_bytes != 0 {
            show_warning!("invalid width {}; using {} instead", line_bytes, min_bytes);
            line_bytes = min_bytes;
        }

        let output_duplicates = matches.opt_present("v");

        odfunc(line_bytes, &input_offset_base, byte_order, &inputs, &formats[..], output_duplicates)
}

fn odfunc(line_bytes: usize, input_offset_base: &Radix, byte_order: ByteOrder,
        fnames: &[InputSource], formats: &[FormatterItemInfo], output_duplicates: bool) -> i32 {

    let mut mf = MultifileReader::new(fnames);
    let mut addr = 0;
    let mut bytes: Vec<u8> = vec![b'\x00'; line_bytes];
    let mut previous_bytes = Vec::<u8>::with_capacity(line_bytes);
    let mut duplicate_line = false;

    loop {
        // print each line data (or multi-format raster of several lines describing the same data).

        match mf.f_read(bytes.as_mut_slice()) {
            Ok(0) => {
                print!("{}\n", print_with_radix(input_offset_base, addr)); // print final offset
                break;
            }
            Ok(n) => {
                // not enough byte for a whole element, this should only happen on the last line.
                if n != line_bytes {
                    // set zero bytes in the part of the buffer that will be used, but is not filled.
                    for i in n..line_bytes {
                        bytes[i] = 0;
                    }
                }

                if !output_duplicates && previous_bytes == bytes && n == line_bytes {
                    if !duplicate_line {
                        duplicate_line = true;
                        println!("*");
                    }
                }
                else {
                    duplicate_line = false;
                    previous_bytes.clone_from(&bytes);

                    print_bytes(byte_order, &bytes, n, &print_with_radix(input_offset_base, addr), formats);
                }

                addr += n;
            }
            Err(_) => {
                break;
            }
        };
    }

    if mf.any_err {
        1
    } else {
        0
    }
}

fn print_bytes(byte_order: ByteOrder, bytes: &[u8], length: usize, prefix: &str, formats: &[FormatterItemInfo]) {
    let mut first = true; // First line of a multi-format raster.
    for f in formats {
        let mut output_text = String::new();

        let mut b = 0;
        while b < length {
            let nextb = b + f.byte_size;
            match f.formatter {
                FormatWriter::IntWriter(func) => {
                    let p: u64 = match f.byte_size {
                        1 => {
                            bytes[b] as u64
                        }
                        2 => {
                            byte_order.read_u16(&bytes[b..nextb]) as u64
                        }
                        4 => {
                            byte_order.read_u32(&bytes[b..nextb]) as u64
                        }
                        8 => {
                            byte_order.read_u64(&bytes[b..nextb])
                        }
                        _ => { panic!("Invalid byte_size: {}", f.byte_size); }
                    };
                    output_text.push_str(&func(p, f.byte_size, f.print_width));
                }
                FormatWriter::FloatWriter(func) => {
                    let p: f64 = match f.byte_size {
                        4 => {
                            byte_order.read_f32(&bytes[b..nextb]) as f64
                        }
                        8 => {
                            byte_order.read_f64(&bytes[b..nextb])
                        }
                        _ => { panic!("Invalid byte_size: {}", f.byte_size); }
                    };
                    output_text.push_str(&func(p));
                }
            }
            b = nextb;
        }

        if first {
            print!("{}", prefix); // print offset
            // if printing in multiple formats offset is printed only once
            first = false;
        }
        else {
            // this takes the space of the file offset on subsequent
            // lines of multi-format rasters.
            print!("{:>width$}", "", width=prefix.chars().count());
        }
        print!("{}\n", output_text);
    }
}

// For file byte offset printed at left margin.
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

fn print_with_radix(r: &Radix, x: usize) -> String{
    // TODO(keunwoo): field widths should be based on sizeof(x), or chosen dynamically based on the
    // expected range of address values.  Binary in particular is not great here.
    match *r {
        Radix::Decimal => format!("{:07}", x),
        Radix::Hexadecimal => format!("{:07X}", x),
        Radix::Octal => format!("{:07o}", x),
        Radix::Binary => format!("{:07b}", x)
    }
}
