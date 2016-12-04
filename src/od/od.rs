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
extern crate half;

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
mod output_info;
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
use output_info::OutputInfo;

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
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

fn create_getopts_options() -> getopts::Options {
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
    opts.optflag("", "help", "display this help and exit.");
    opts.optflag("", "version", "output version information and exit.");
    opts.optflag("", "traditional", "compatibility mode with one input, offset and label.");

    opts
}

struct OdOptions {
    byte_order: ByteOrder,
    skip_bytes: usize,
    read_bytes: Option<usize>,
    label: Option<usize>,
    input_strings: Vec<String>,
    formats: Vec<ParsedFormatterItemInfo>,
    line_bytes: usize,
    output_duplicates: bool,
    radix: Radix,
}

impl OdOptions {
    fn new(matches: getopts::Matches, args: Vec<String>) -> Result<OdOptions, String> {
        let byte_order = match matches.opt_str("endian").as_ref().map(String::as_ref) {
            None => { ByteOrder::Native },
            Some("little") => { ByteOrder::Little },
            Some("big") => { ByteOrder::Big },
            Some(s) => {
                return Err(format!("Invalid argument --endian={}", s));
            }
        };

        let mut skip_bytes = match matches.opt_default("skip-bytes", "0") {
            None => 0,
            Some(s) => {
                match parse_number_of_bytes(&s) {
                    Ok(i) => { i }
                    Err(_) => {
                        return Err(format!("Invalid argument --skip-bytes={}", s));
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
                vec![f]
            },
            Err(e) => {
                return Err(format!("Invalid inputs: {}", e));
            }
        };

        let formats = match parse_format_flags(&args) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("{}", e));
            }
        };

        let mut line_bytes = match matches.opt_default("w", "32") {
            None => 16,
            Some(s) => {
                match s.parse::<usize>() {
                    Ok(i) => { i }
                    Err(_) => { 0 }
                }
            }
        };
        let min_bytes = formats.iter().fold(1, |max, next| cmp::max(max, next.formatter_item_info.byte_size));
        if line_bytes == 0 || line_bytes % min_bytes != 0 {
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
                        return Err(format!("Invalid argument --read-bytes={}", s));
                    }
                }
            }
        };

        let radix = match matches.opt_str("A") {
            None => Radix::Octal,
            Some(s) => {
                let st = s.into_bytes();
                if st.len() != 1 {
                    return Err(format!("Radix must be one of [d, o, n, x]"))
                } else {
                    let radix: char = *(st.get(0)
                                          .expect("byte string of length 1 lacks a 0th elem")) as char;
                    match radix {
                        'd' => Radix::Decimal,
                        'x' => Radix::Hexadecimal,
                        'o' => Radix::Octal,
                        'n' => Radix::NoPrefix,
                        _ => return Err(format!("Radix must be one of [d, o, n, x]"))
                    }
                }
            }
        };

        Ok(OdOptions {
            byte_order: byte_order,
            skip_bytes: skip_bytes,
            read_bytes: read_bytes,
            label: label,
            input_strings: input_strings,
            formats: formats,
            line_bytes: line_bytes,
            output_duplicates: output_duplicates,
            radix: radix,
        })
    }
}

/// parses and validates commandline parameters, prepares data structures,
/// opens the input and calls `odfunc` to process the input.
pub fn uumain(args: Vec<String>) -> i32 {
    let opts = create_getopts_options();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            disp_err!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("{}", opts.usage(&USAGE));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", executable!(), VERSION);
        return 0;
    }

    let od_options = match OdOptions::new(matches, args) {
        Err(s) => {
            disp_err!("{}", s);
            return 1;
        },
        Ok(o) => o,
    };

    let mut input_offset = InputOffset::new(od_options.radix, od_options.skip_bytes,
        od_options.label);

    let mut input = open_input_peek_reader(&od_options.input_strings,
        od_options.skip_bytes, od_options.read_bytes);
    let mut input_decoder = InputDecoder::new(&mut input, od_options.line_bytes,
        PEEK_BUFFER_SIZE, od_options.byte_order);

    let output_info = OutputInfo::new(od_options.line_bytes, &od_options.formats[..],
        od_options.output_duplicates);

    odfunc(&mut input_offset, &mut input_decoder, &output_info)
}

/// Loops through the input line by line, calling print_bytes to take care of the output.
fn odfunc<I>(input_offset: &mut InputOffset, input_decoder: &mut InputDecoder<I>,
        output_info: &OutputInfo) -> i32
        where I: PeekRead + HasError {
    let mut duplicate_line = false;
    let mut previous_bytes: Vec<u8> = Vec::new();
    let line_bytes = output_info.byte_size_line;

    loop {
        // print each line data (or multi-format raster of several lines describing the same data).

        match input_decoder.peek_read() {
            Ok(mut memory_decoder) => {
                let length = memory_decoder.length();

                if length == 0 {
                    input_offset.print_final_offset();
                    break;
                }

                // not enough byte for a whole element, this should only happen on the last line.
                if length != line_bytes {
                    // set zero bytes in the part of the buffer that will be used, but is not filled.
                    let mut max_used = length + output_info.byte_size_block;
                    if max_used > line_bytes {
                        max_used = line_bytes;
                    }

                    memory_decoder.zero_out_buffer(length, max_used);
                }

                if !output_info.output_duplicates
                        && length == line_bytes
                        && memory_decoder.get_buffer(0) == &previous_bytes[..] {
                    if !duplicate_line {
                        duplicate_line = true;
                        println!("*");
                    }
                } else {
                    duplicate_line = false;
                    if length == line_bytes {
                        // save a copy of the input unless it is the last line
                        memory_decoder.clone_buffer(&mut previous_bytes);
                    }

                    print_bytes(&input_offset.format_byte_offset(), &memory_decoder,
                        &output_info);
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

/// Outputs a single line of input, into one or more lines human readable output.
fn print_bytes(prefix: &str, input_decoder: &MemoryDecoder, output_info: &OutputInfo) {
    let mut first = true; // First line of a multi-format raster.
    for f in output_info.spaced_formatters_iter() {
        let mut output_text = String::new();

        let mut b = 0;
        while b < input_decoder.length() {
            output_text.push_str(&format!("{:>width$}",
                    "",
                    width = f.spacing[b % output_info.byte_size_block]));

            match f.formatter_item_info.formatter {
                FormatWriter::IntWriter(func) => {
                    let p = input_decoder.read_uint(b, f.formatter_item_info.byte_size);
                    output_text.push_str(&func(p));
                }
                FormatWriter::FloatWriter(func) => {
                    let p = input_decoder.read_float(b, f.formatter_item_info.byte_size);
                    output_text.push_str(&func(p));
                }
                FormatWriter::MultibyteWriter(func) => {
                    output_text.push_str(&func(input_decoder.get_full_buffer(b)));
                }
            }

            b += f.formatter_item_info.byte_size;
        }

        if f.add_ascii_dump {
            let missing_spacing = output_info.print_width_line.saturating_sub(output_text.chars().count());
            output_text.push_str(&format!("{:>width$}  {}",
                    "",
                    format_ascii_dump(input_decoder.get_buffer(0)),
                    width = missing_spacing));
        }

        if first {
            print!("{}", prefix); // print offset
            // if printing in multiple formats offset is printed only once
            first = false;
        } else {
            // this takes the space of the file offset on subsequent
            // lines of multi-format rasters.
            print!("{:>width$}", "", width=prefix.chars().count());
        }
        print!("{}\n", output_text);
    }
}

/// returns a reader implementing `PeekRead + Read + HasError` providing the combined input
///
/// `skip_bytes` is the number of bytes skipped from the input
/// `read_bytes` is an optional limit to the number of bytes to read
fn open_input_peek_reader<'a>(input_strings: &'a Vec<String>, skip_bytes: usize,
        read_bytes: Option<usize>) -> PeekReader<PartialReader<MultifileReader<'a>>> {
    // should return  "impl PeekRead + Read + HasError" when supported in (stable) rust
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
