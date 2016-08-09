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
mod partialreader;
mod peekreader;
mod byteorder_io;
mod formatteriteminfo;
mod prn_int;
mod prn_char;
mod prn_float;
mod parse_nrofbytes;
mod parse_formats;
#[cfg(test)]
mod mockstream;

use std::cmp;
use std::io::Write;
use unindent::*;
use byteorder_io::*;
use multifilereader::*;
use partialreader::*;
use peekreader::*;
use formatteriteminfo::*;
use parse_nrofbytes::*;
use parse_formats::parse_format_flags;

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
const MAX_BYTES_PER_UNIT: usize = 8;
const PEEK_BUFFER_SIZE: usize = 4; // utf-8 can be 4 bytes

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Radix { Decimal, Hexadecimal, Octal, NoPrefix }

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
    opts.optflagmulti("a", "", "named characters, ignoring high-order bit");
    opts.optflagmulti("b", "", "octal bytes");
    opts.optflagmulti("c", "", "ASCII characters or backslash escapes");
    opts.optflagmulti("d", "", "unsigned decimal 2-byte units");
    opts.optflagmulti("D", "", "unsigned decimal 4-byte units");
    opts.optflagmulti("o", "", "octal 2-byte units");

    opts.optflagmulti("I", "", "decimal 8-byte units");
    opts.optflagmulti("L", "", "decimal 8-byte units");
    opts.optflagmulti("i", "", "decimal 4-byte units");
    opts.optflagmulti("l", "", "decimal 8-byte units");
    opts.optflagmulti("x", "", "hexadecimal 2-byte units");
    opts.optflagmulti("h", "", "hexadecimal 2-byte units");

    opts.optflagmulti("O", "", "octal 4-byte units");
    opts.optflagmulti("s", "", "decimal 2-byte units");
    opts.optflagmulti("X", "", "hexadecimal 4-byte units");
    opts.optflagmulti("H", "", "hexadecimal 4-byte units");

    opts.optflagmulti("e", "", "floating point double precision (64-bit) units");
    opts.optflagmulti("f", "", "floating point single precision (32-bit) units");
    opts.optflagmulti("F", "", "floating point double precision (64-bit) units");

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
    let mut inputs = args[1..]
        .iter()
        .filter_map(|w| match w as &str {
            "--" => Some(InputSource::Stdin),
            o if o.starts_with("-") => None,
            x => Some(InputSource::FileName(x)),
        })
        .collect::<Vec<_>>();
    if inputs.len() == 0 {
        inputs.push(InputSource::Stdin);
    }

    let formats = parse_format_flags(&args);

    let mut line_bytes = match matches.opt_default("w", "32") {
        None => 16,
        Some(s) => {
            match s.parse::<usize>() {
                Ok(i) => { i }
                Err(_) => { 2 }
            }
        }
    };
    let min_bytes = formats.iter().fold(1, |max, next| cmp::max(max, next.byte_size));
    if line_bytes % min_bytes != 0 {
        show_warning!("invalid width {}; using {} instead", line_bytes, min_bytes);
        line_bytes = min_bytes;
    }

    let output_duplicates = matches.opt_present("v");

    let skip_bytes = match matches.opt_default("skip-bytes", "0") {
        None => 0,
        Some(s) => {
            match parse_number_of_bytes(&s) {
                Ok(i) => { i }
                Err(_) => {
                    disp_err!("Invalid argument --skip-bytes={}", s);
                    return 1;
                }
            }
        }
    };
    let read_bytes = match matches.opt_str("read-bytes") {
        None => None,
        Some(s) => {
            match  parse_number_of_bytes(&s) {
                Ok(i) => { Some(i) }
                Err(_) => {
                    disp_err!("Invalid argument --read-bytes={}", s);
                    return 1;
                }
            }
        }
    };

    odfunc(line_bytes, input_offset_base, byte_order, inputs, &formats[..],
            output_duplicates, skip_bytes, read_bytes)
}

fn odfunc(line_bytes: usize, input_offset_base: Radix, byte_order: ByteOrder,
        fnames: Vec<InputSource>, formats: &[FormatterItemInfo], output_duplicates: bool,
        skip_bytes: usize, read_bytes: Option<usize>) -> i32 {

    let mf = MultifileReader::new(fnames);
    let pr = PartialReader::new(mf, skip_bytes, read_bytes);
    let mut input = PeekReader::new(pr);
    let mut addr = skip_bytes;
    let mut duplicate_line = false;
    let mut previous_bytes: Vec<u8> = Vec::new();
    let mut bytes: Vec<u8> = Vec::with_capacity(line_bytes + PEEK_BUFFER_SIZE);
    unsafe { bytes.set_len(line_bytes + PEEK_BUFFER_SIZE); } // fast but uninitialized

    let byte_size_block = formats.iter().fold(1, |max, next| cmp::max(max, next.byte_size));
    let print_width_block = formats
        .iter()
        .fold(1, |max, next| {
            cmp::max(max, next.print_width * (byte_size_block / next.byte_size))
        });

    if byte_size_block > MAX_BYTES_PER_UNIT {
        panic!("{}-bits types are unsupported. Current max={}-bits.",
                8 * byte_size_block,
                8 * MAX_BYTES_PER_UNIT);
    }

    let mut spaced_formatters: Vec<SpacedFormatterItemInfo> = formats
        .iter()
        .map(|f| SpacedFormatterItemInfo { frm: *f, spacing: [0; MAX_BYTES_PER_UNIT] })
        .collect();

    // calculate proper alignment for each item
    for sf in &mut spaced_formatters {
        let mut byte_size = sf.frm.byte_size;
        let mut items_in_block = byte_size_block / byte_size;
        let thisblock_width = sf.frm.print_width * items_in_block;
        let mut missing_spacing = print_width_block - thisblock_width;

        while items_in_block > 0 {
            let avg_spacing: usize = missing_spacing / items_in_block;
            for i in 0..items_in_block {
                sf.spacing[i * byte_size] += avg_spacing;
                missing_spacing -= avg_spacing;
            }
            // this assumes the size of all types is a power of 2 (1, 2, 4, 8, 16, ...)
            items_in_block /= 2;
            byte_size *= 2;
        }
    }

    loop {
        // print each line data (or multi-format raster of several lines describing the same data).
        // TODO: we need to read more data in case a multi-byte sequence starts at the end of the line

        match input.peek_read(bytes.as_mut_slice(), PEEK_BUFFER_SIZE) {
            Ok((0, _)) => {
                print_final_offset(input_offset_base, addr);
                break;
            }
            Ok((n, peekbytes)) => {
                // not enough byte for a whole element, this should only happen on the last line.
                if n != line_bytes {
                    // set zero bytes in the part of the buffer that will be used, but is not filled.
                    let mut max_used = n + MAX_BYTES_PER_UNIT;
                    if max_used > line_bytes {
                        max_used = line_bytes;
                    }

                    for i in n..max_used {
                        bytes[i] = 0;
                    }
                }

                if !output_duplicates
                        && n == line_bytes
                        && !previous_bytes.is_empty()
                        && previous_bytes[..line_bytes] == bytes[..line_bytes] {
                    if !duplicate_line {
                        duplicate_line = true;
                        println!("*");
                    }
                }
                else {
                    duplicate_line = false;
                    if n == line_bytes {
                        // save a copy of the input unless it is the last line
                        previous_bytes.clone_from(&bytes);
                    }

                    print_bytes(byte_order, &bytes, n, peekbytes,
                        &print_with_radix(input_offset_base, addr),
                        &spaced_formatters, byte_size_block);
                }

                addr += n;
            }
            Err(e) => {
                show_error!("{}", e);
                print_final_offset(input_offset_base, addr);
                return 1;
            }
        };
    }

    if input.has_error() {
        1
    } else {
        0
    }
}

fn print_bytes(byte_order: ByteOrder, bytes: &[u8], length: usize, peekbytes: usize, prefix: &str,
        formats: &[SpacedFormatterItemInfo], byte_size_block: usize) {
    let mut first = true; // First line of a multi-format raster.
    for f in formats {
        let mut output_text = String::new();

        let mut b = 0;
        while b < length {
            let nextb = b + f.frm.byte_size;

            output_text.push_str(&format!("{:>width$}",
                    "",
                    width = f.spacing[b % byte_size_block]));

            match f.frm.formatter {
                FormatWriter::IntWriter(func) => {
                    let p: u64 = match f.frm.byte_size {
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
                        _ => { panic!("Invalid byte_size: {}", f.frm.byte_size); }
                    };
                    output_text.push_str(&func(p, f.frm.byte_size, f.frm.print_width));
                }
                FormatWriter::FloatWriter(func) => {
                    let p: f64 = match f.frm.byte_size {
                        4 => {
                            byte_order.read_f32(&bytes[b..nextb]) as f64
                        }
                        8 => {
                            byte_order.read_f64(&bytes[b..nextb])
                        }
                        _ => { panic!("Invalid byte_size: {}", f.frm.byte_size); }
                    };
                    output_text.push_str(&func(p));
                }
                FormatWriter::MultibyteWriter(func) => {
                    output_text.push_str(&func(&bytes[b..length+peekbytes]));
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
                Err("Radix must be one of [d, o, n, x]\n")
            } else {
                let radix: char = *(st.get(0)
                                      .expect("byte string of length 1 lacks a 0th elem")) as char;
                match radix {
                    'd' => Ok(Radix::Decimal),
                    'x' => Ok(Radix::Hexadecimal),
                    'o' => Ok(Radix::Octal),
                    'n' => Ok(Radix::NoPrefix),
                    _ => Err("Radix must be one of [d, o, n, x]\n")
                }
            }
        }
    }
}

fn print_with_radix(r: Radix, x: usize) -> String{
    match r {
        Radix::Decimal => format!("{:07}", x),
        Radix::Hexadecimal => format!("{:06X}", x),
        Radix::Octal => format!("{:07o}", x),
        Radix::NoPrefix => String::from(""),
    }
}

fn print_final_offset(r: Radix, x: usize) {
    if r != Radix::NoPrefix {
        print!("{}\n", print_with_radix(r, x));
    }
}

struct SpacedFormatterItemInfo {
    frm: FormatterItemInfo,
    spacing: [usize; MAX_BYTES_PER_UNIT],
}
