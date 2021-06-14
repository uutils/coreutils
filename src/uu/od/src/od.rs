//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Ben Hirsch <benhirsch24@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) formatteriteminfo inputdecoder inputoffset mockstream nrofbytes partialreader odfunc multifile exitcode

#[macro_use]
extern crate uucore;

pub mod app;
mod byteorder_io;
mod formatteriteminfo;
mod inputdecoder;
mod inputoffset;
#[cfg(test)]
mod mockstream;
mod multifilereader;
mod output_info;
mod parse_formats;
mod parse_inputs;
mod parse_nrofbytes;
mod partialreader;
mod peekreader;
mod prn_char;
mod prn_float;
mod prn_int;

use std::cmp;

use crate::app::get_app;
use crate::app::options;
use crate::byteorder_io::*;
use crate::formatteriteminfo::*;
use crate::inputdecoder::{InputDecoder, MemoryDecoder};
use crate::inputoffset::{InputOffset, Radix};
use crate::multifilereader::*;
use crate::output_info::OutputInfo;
use crate::parse_formats::{parse_format_flags, ParsedFormatterItemInfo};
use crate::parse_inputs::{parse_inputs, CommandLineInputs};
use crate::parse_nrofbytes::parse_number_of_bytes;
use crate::partialreader::*;
use crate::peekreader::*;
use crate::prn_char::format_ascii_dump;
use clap::ArgMatches;
use uucore::parse_size::ParseSizeError;
use uucore::InvalidEncodingHandling;

const PEEK_BUFFER_SIZE: usize = 4; // utf-8 can be 4 bytes

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
    fn new(matches: ArgMatches, args: Vec<String>) -> Result<OdOptions, String> {
        let byte_order = match matches.value_of(options::ENDIAN) {
            None => ByteOrder::Native,
            Some("little") => ByteOrder::Little,
            Some("big") => ByteOrder::Big,
            Some(s) => {
                return Err(format!("Invalid argument --endian={}", s));
            }
        };

        let mut skip_bytes = matches.value_of(options::SKIP_BYTES).map_or(0, |s| {
            parse_number_of_bytes(s).unwrap_or_else(|e| {
                crash!(1, "{}", format_error_message(e, s, options::SKIP_BYTES))
            })
        });

        let mut label: Option<usize> = None;

        let parsed_input = parse_inputs(&matches).map_err(|e| format!("Invalid inputs: {}", e))?;
        let input_strings = match parsed_input {
            CommandLineInputs::FileNames(v) => v,
            CommandLineInputs::FileAndOffset((f, s, l)) => {
                skip_bytes = s;
                label = l;
                vec![f]
            }
        };

        let formats = parse_format_flags(&args)?;

        let mut line_bytes = matches.value_of(options::WIDTH).map_or(16, |s| {
            if matches.occurrences_of(options::WIDTH) == 0 {
                return 16;
            };
            parse_number_of_bytes(s)
                .unwrap_or_else(|e| crash!(1, "{}", format_error_message(e, s, options::WIDTH)))
        });

        let min_bytes = formats.iter().fold(1, |max, next| {
            cmp::max(max, next.formatter_item_info.byte_size)
        });
        if line_bytes == 0 || line_bytes % min_bytes != 0 {
            show_warning!("invalid width {}; using {} instead", line_bytes, min_bytes);
            line_bytes = min_bytes;
        }

        let output_duplicates = matches.is_present(options::OUTPUT_DUPLICATES);

        let read_bytes = matches.value_of(options::READ_BYTES).map(|s| {
            parse_number_of_bytes(s).unwrap_or_else(|e| {
                crash!(1, "{}", format_error_message(e, s, options::READ_BYTES))
            })
        });

        let radix = match matches.value_of(options::ADDRESS_RADIX) {
            None => Radix::Octal,
            Some(s) => {
                let st = s.as_bytes();
                if st.len() != 1 {
                    return Err("Radix must be one of [d, o, n, x]".to_string());
                } else {
                    let radix: char =
                        *(st.get(0).expect("byte string of length 1 lacks a 0th elem")) as char;
                    match radix {
                        'd' => Radix::Decimal,
                        'x' => Radix::Hexadecimal,
                        'o' => Radix::Octal,
                        'n' => Radix::NoPrefix,
                        _ => return Err("Radix must be one of [d, o, n, x]".to_string()),
                    }
                }
            }
        };

        Ok(OdOptions {
            byte_order,
            skip_bytes,
            read_bytes,
            label,
            input_strings,
            formats,
            line_bytes,
            output_duplicates,
            radix,
        })
    }
}

/// parses and validates command line parameters, prepares data structures,
/// opens the input and calls `odfunc` to process the input.
pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let clap_opts = get_app(executable!());

    let clap_matches = clap_opts
        .clone() // Clone to reuse clap_opts to print help
        .get_matches_from(args.clone());

    let od_options = match OdOptions::new(clap_matches, args) {
        Err(s) => {
            crash!(1, "{}", s);
        }
        Ok(o) => o,
    };

    let mut input_offset =
        InputOffset::new(od_options.radix, od_options.skip_bytes, od_options.label);

    let mut input = open_input_peek_reader(
        &od_options.input_strings,
        od_options.skip_bytes,
        od_options.read_bytes,
    );
    let mut input_decoder = InputDecoder::new(
        &mut input,
        od_options.line_bytes,
        PEEK_BUFFER_SIZE,
        od_options.byte_order,
    );

    let output_info = OutputInfo::new(
        od_options.line_bytes,
        &od_options.formats[..],
        od_options.output_duplicates,
    );

    odfunc(&mut input_offset, &mut input_decoder, &output_info)
}

/// Loops through the input line by line, calling print_bytes to take care of the output.
fn odfunc<I>(
    input_offset: &mut InputOffset,
    input_decoder: &mut InputDecoder<I>,
    output_info: &OutputInfo,
) -> i32
where
    I: PeekRead + HasError,
{
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
                    && memory_decoder.get_buffer(0) == &previous_bytes[..]
                {
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

                    print_bytes(
                        &input_offset.format_byte_offset(),
                        &memory_decoder,
                        output_info,
                    );
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
            output_text.push_str(&format!(
                "{:>width$}",
                "",
                width = f.spacing[b % output_info.byte_size_block]
            ));

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
            let missing_spacing = output_info
                .print_width_line
                .saturating_sub(output_text.chars().count());
            output_text.push_str(&format!(
                "{:>width$}  {}",
                "",
                format_ascii_dump(input_decoder.get_buffer(0)),
                width = missing_spacing
            ));
        }

        if first {
            print!("{}", prefix); // print offset
                                  // if printing in multiple formats offset is printed only once
            first = false;
        } else {
            // this takes the space of the file offset on subsequent
            // lines of multi-format rasters.
            print!("{:>width$}", "", width = prefix.chars().count());
        }
        println!("{}", output_text);
    }
}

/// returns a reader implementing `PeekRead + Read + HasError` providing the combined input
///
/// `skip_bytes` is the number of bytes skipped from the input
/// `read_bytes` is an optional limit to the number of bytes to read
fn open_input_peek_reader(
    input_strings: &[String],
    skip_bytes: usize,
    read_bytes: Option<usize>,
) -> PeekReader<PartialReader<MultifileReader>> {
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
    PeekReader::new(pr)
}

fn format_error_message(error: ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's od echos affected flag, -N or --read-bytes (-j or --skip-bytes, etc.), depending user's selection
    // GNU's od does distinguish between "invalid (suffix in) argument"
    match error {
        ParseSizeError::ParseFailure(_) => format!("invalid --{} argument '{}'", option, s),
        ParseSizeError::SizeTooBig(_) => format!("--{} argument '{}' too large", option, s),
    }
}
