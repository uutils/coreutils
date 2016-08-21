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
mod parse_inputs;
mod inputoffset;
mod inputdecoder;
#[cfg(test)]
mod mockstream;

use std::cmp;
use std::io::Write;
use byteorder_io::*;
use multifilereader::*;
use partialreader::*;
use peekreader::*;
use formatteriteminfo::*;
use parse_nrofbytes::parse_number_of_bytes;
use parse_formats::{parse_format_flags, ParsedFormatterItemInfo};
use prn_char::format_ascii_dump;
use parse_inputs::{parse_inputs, CommandLineInputs};
use inputoffset::{InputOffset, Radix};
use inputdecoder::{InputDecoder,MemoryDecoder};

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
const MAX_BYTES_PER_UNIT: usize = 8;
const PEEK_BUFFER_SIZE: usize = 4; // utf-8 can be 4 bytes

static USAGE: &'static str =
r#"Usage:
    od [OPTION]... [--] [FILENAME]...
    od [-abcdDefFhHiIlLoOsxX] [FILENAME] [[+][0x]OFFSET[.][b]]
    od --traditional [OPTION]... [FILENAME] [[+][0x]OFFSET[.][b] [[+][0x]LABEL[.][b]]]

Displays data in various human-readable formats. If multiple formats are
specified, the output will contain all formats in the order they appear on the
commandline. Each format will be printed on a new line. Only the line
containing the first format will be prefixed with the offset.

If no filename is specified, or it is "-", stdin will be used. After a "--", no
more options will be recognised. This allows for filenames starting with a "-".

If a filename is a valid number which can be used as an offset in the second
form, you can force it to be recognised as a filename if you include an option
like "-j0", which is only valid in the first form.

RADIX is one of o,d,x,n for octal, decimal, hexadecimal or none.

BYTES is decimal by default, octal if prefixed with a "0", or hexadecimal if
prefixed with "0x". The suffixes b, KB, K, MB, M, GB, G, will multiply the
number with 512, 1000, 1024, 1000^2, 1024^2, 1000^3, 1024^3, 1000^2, 1024^2.

OFFSET and LABEL are octal by default, hexadecimal if prefixed with "0x" or
decimal if a "." suffix is added. The "b" suffix will multiply with 512.

TYPE contains one or more format specifications consisting of:
    a       for printable 7-bits ASCII
    c       for utf-8 characters or octal for undefined characters
    d[SIZE] for signed decimal
    f[SIZE] for floating point
    o[SIZE] for octal
    u[SIZE] for unsigned decimal
    x[SIZE] for hexadecimal
SIZE is the number of bytes which can be the number 1, 2, 4, 8 or 16,
    or C, I, S, L for 1, 2, 4, 8 bytes for integer types,
    or F, D, L for 4, 8, 16 bytes for floating point.
Any type specification can have a "z" suffic, which will add a ASCII dump at
    the end of the line.

If an error occurred, a diagnostic message will be printed to stderr, and the
exitcode will be non-zero."#;

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

    opts.optmulti("t", "format", "select output format or formats", "TYPE");
    opts.optflag("v", "output-duplicates", "do not use * to mark line suppression");
    opts.optflagopt("w", "width",
                ("output BYTES bytes per output line. 32 is implied when BYTES is not \
                 specified."),
                "BYTES");
    opts.optflag("h", "help", "display this help and exit.");
    opts.optflag("", "version", "output version information and exit.");
    opts.optflag("", "traditional", "compatibility mode with one input, offset and label.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            disp_err!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("h") {
        println!("{}", opts.usage(&USAGE));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", executable!(), VERSION);
        return 0;
    }

    let byte_order = match matches.opt_str("endian").as_ref().map(String::as_ref) {
        None => { ByteOrder::Native },
        Some("little") => { ByteOrder::Little },
        Some("big") => { ByteOrder::Big },
        Some(s) => {
            disp_err!("Invalid argument --endian={}", s);
            return 1;
        }
    };

    let mut skip_bytes = match matches.opt_default("skip-bytes", "0") {
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

    let mut label: Option<usize> = None;

    let input_strings = match parse_inputs(&matches) {
        Ok(CommandLineInputs::FileNames(v)) => v,
        Ok(CommandLineInputs::FileAndOffset((f, s, l))) => {
            skip_bytes = s;
            label = l;
            vec!{f}
        },
        Err(e) => {
            disp_err!("Invalid inputs: {}", e);
            return 1;
        }
    };

    let formats = match parse_format_flags(&args) {
        Ok(f) => f,
        Err(e) => {
            disp_err!("{}", e);
            return 1;
        }
    };

    let mut line_bytes = match matches.opt_default("w", "32") {
        None => 16,
        Some(s) => {
            match s.parse::<usize>() {
                Ok(i) => { i }
                Err(_) => { 2 }
            }
        }
    };
    let min_bytes = formats.iter().fold(1, |max, next| cmp::max(max, next.formatter_item_info.byte_size));
    if line_bytes % min_bytes != 0 {
        show_warning!("invalid width {}; using {} instead", line_bytes, min_bytes);
        line_bytes = min_bytes;
    }

    let output_duplicates = matches.opt_present("v");

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

    let mut input = open_input_peek_reader(&input_strings, skip_bytes, read_bytes);

    let mut input_decoder = InputDecoder::new(&mut input, line_bytes, PEEK_BUFFER_SIZE, byte_order);

    let mut input_offset = InputOffset::new(Radix::Octal, skip_bytes, label);
    if let Err(e) = input_offset.parse_radix_from_commandline(matches.opt_str("A")) {
        disp_err!("Invalid -A/--address-radix\n{}", e);
        return 1;
    }

    odfunc(&mut input_decoder, &mut input_offset, line_bytes, &formats[..],
            output_duplicates)
}

// TODO: refactor, too many arguments
fn odfunc<I>(input_decoder: &mut InputDecoder<I>, input_offset: &mut InputOffset, line_bytes: usize,
        formats: &[ParsedFormatterItemInfo], output_duplicates: bool) -> i32
        where I : PeekRead+HasError {

    let mut duplicate_line = false;
    let mut previous_bytes: Vec<u8> = Vec::new();

    let byte_size_block = formats.iter().fold(1, |max, next| cmp::max(max, next.formatter_item_info.byte_size));
    let print_width_block = formats
        .iter()
        .fold(1, |max, next| {
            cmp::max(max, next.formatter_item_info.print_width * (byte_size_block / next.formatter_item_info.byte_size))
        });
    let print_width_line = print_width_block * (line_bytes / byte_size_block);

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
        let mut byte_size = sf.frm.formatter_item_info.byte_size;
        let mut items_in_block = byte_size_block / byte_size;
        let thisblock_width = sf.frm.formatter_item_info.print_width * items_in_block;
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

        match input_decoder.peek_read() {
            Ok(mut memory_decoder) => {
                let length=memory_decoder.length();

                if length == 0 {
                    input_offset.print_final_offset();
                    break;
                }

                // not enough byte for a whole element, this should only happen on the last line.
                if length != line_bytes {
                    // set zero bytes in the part of the buffer that will be used, but is not filled.
                    let mut max_used = length + MAX_BYTES_PER_UNIT;
                    if max_used > line_bytes {
                        max_used = line_bytes;
                    }

                    memory_decoder.zero_out_buffer(length, max_used);
                }

                if !output_duplicates
                        && length == line_bytes
                        && memory_decoder.get_buffer(0) == &previous_bytes[..] {
                    if !duplicate_line {
                        duplicate_line = true;
                        println!("*");
                    }
                }
                else {
                    duplicate_line = false;
                    if length == line_bytes {
                        // save a copy of the input unless it is the last line
                        memory_decoder.clone_buffer(&mut previous_bytes);
                    }

                    print_bytes(&input_offset.format_byte_offset(), &memory_decoder,
                        &spaced_formatters, byte_size_block, print_width_line);
                }

                input_offset.increase_position(length);
            }
            Err(e) => {
                show_error!("{}", e);
                input_offset.print_final_offset();
                return 1;
            }
        };
    }

    if input_decoder.has_error() {
        1
    } else {
        0
    }
}

fn print_bytes(prefix: &str, input_decoder: &MemoryDecoder,
        formats: &[SpacedFormatterItemInfo], byte_size_block: usize, print_width_line: usize) {
    let mut first = true; // First line of a multi-format raster.
    for f in formats {
        let mut output_text = String::new();

        let mut b = 0;
        while b < input_decoder.length() {
            output_text.push_str(&format!("{:>width$}",
                    "",
                    width = f.spacing[b % byte_size_block]));

            match f.frm.formatter_item_info.formatter {
                FormatWriter::IntWriter(func) => {
                    let p = input_decoder.read_uint(b, f.frm.formatter_item_info.byte_size);
                    output_text.push_str(&func(p));
                }
                FormatWriter::FloatWriter(func) => {
                    let p = input_decoder.read_float(b, f.frm.formatter_item_info.byte_size);
                    output_text.push_str(&func(p));
                }
                FormatWriter::MultibyteWriter(func) => {
                    output_text.push_str(&func(input_decoder.get_full_buffer(b)));
                }
            }

            b += f.frm.formatter_item_info.byte_size;
        }

        if f.frm.add_ascii_dump {
            let missing_spacing = print_width_line.saturating_sub(output_text.chars().count());
            output_text.push_str(&format!("{:>width$}  {}",
                    "",
                    format_ascii_dump(input_decoder.get_buffer(0)),
                    width=missing_spacing));
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

/// returns a reader implementing `PeekRead+Read+HasError` providing the combined input
///
/// `skip_bytes` is the number of bytes skipped from the input
/// `read_bytes` is an optinal limit to the number of bytes to read
fn open_input_peek_reader<'a>(input_strings: &'a Vec<String>, skip_bytes: usize,
        read_bytes: Option<usize>) -> PeekReader<PartialReader<MultifileReader<'a>>> {
    // should return  "impl PeekRead+Read+HasError" when supported in (stable) rust
    let inputs = input_strings
        .iter()
        .map(|w| match w as &str {
            "-" => InputSource::Stdin,
            x => InputSource::FileName(x),
        })
        .collect::<Vec<_>>();

    let mf = MultifileReader::new(inputs);
    let pr = PartialReader::new(mf, skip_bytes, read_bytes);
    let input = PeekReader::new(pr);
    input
}

struct SpacedFormatterItemInfo {
    frm: ParsedFormatterItemInfo,
    spacing: [usize; MAX_BYTES_PER_UNIT],
}
