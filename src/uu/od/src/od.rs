// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (clap) dont
// spell-checker:ignore (ToDO) formatteriteminfo inputdecoder inputoffset mockstream nrofbytes partialreader odfunc multifile exitcode

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
use std::fmt::Write;

use crate::byteorder_io::ByteOrder;
use crate::formatteriteminfo::FormatWriter;
use crate::inputdecoder::{InputDecoder, MemoryDecoder};
use crate::inputoffset::{InputOffset, Radix};
use crate::multifilereader::{HasError, InputSource, MultifileReader};
use crate::output_info::OutputInfo;
use crate::parse_formats::{parse_format_flags, ParsedFormatterItemInfo};
use crate::parse_inputs::{parse_inputs, CommandLineInputs};
use crate::parse_nrofbytes::parse_number_of_bytes;
use crate::partialreader::PartialReader;
use crate::peekreader::{PeekRead, PeekReader};
use crate::prn_char::format_ascii_dump;
use clap::ArgAction;
use clap::{crate_version, parser::ValueSource, Arg, ArgMatches, Command};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::parse_size::ParseSizeError;
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage, show_error, show_warning};

const PEEK_BUFFER_SIZE: usize = 4; // utf-8 can be 4 bytes

const ABOUT: &str = help_about!("od.md");
const USAGE: &str = help_usage!("od.md");
const AFTER_HELP: &str = help_section!("after help", "od.md");

pub(crate) mod options {
    pub const HELP: &str = "help";
    pub const ADDRESS_RADIX: &str = "address-radix";
    pub const SKIP_BYTES: &str = "skip-bytes";
    pub const READ_BYTES: &str = "read-bytes";
    pub const ENDIAN: &str = "endian";
    pub const STRINGS: &str = "strings";
    pub const FORMAT: &str = "format";
    pub const OUTPUT_DUPLICATES: &str = "output-duplicates";
    pub const TRADITIONAL: &str = "traditional";
    pub const WIDTH: &str = "width";
    pub const FILENAME: &str = "FILENAME";
}

struct OdOptions {
    byte_order: ByteOrder,
    skip_bytes: u64,
    read_bytes: Option<u64>,
    label: Option<u64>,
    input_strings: Vec<String>,
    formats: Vec<ParsedFormatterItemInfo>,
    line_bytes: usize,
    output_duplicates: bool,
    radix: Radix,
}

impl OdOptions {
    fn new(matches: &ArgMatches, args: &[String]) -> UResult<Self> {
        let byte_order = if let Some(s) = matches.get_one::<String>(options::ENDIAN) {
            match s.as_str() {
                "little" => ByteOrder::Little,
                "big" => ByteOrder::Big,
                _ => {
                    return Err(USimpleError::new(
                        1,
                        format!("Invalid argument --endian={s}"),
                    ))
                }
            }
        } else {
            ByteOrder::Native
        };

        let mut skip_bytes = match matches.get_one::<String>(options::SKIP_BYTES) {
            None => 0,
            Some(s) => match parse_number_of_bytes(s) {
                Ok(n) => n,
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format_error_message(&e, s, options::SKIP_BYTES),
                    ))
                }
            },
        };

        let mut label: Option<u64> = None;

        let parsed_input = parse_inputs(matches)
            .map_err(|e| USimpleError::new(1, format!("Invalid inputs: {e}")))?;
        let input_strings = match parsed_input {
            CommandLineInputs::FileNames(v) => v,
            CommandLineInputs::FileAndOffset((f, s, l)) => {
                skip_bytes = s;
                label = l;
                vec![f]
            }
        };

        let formats = parse_format_flags(args).map_err(|e| USimpleError::new(1, e))?;

        let mut line_bytes = match matches.get_one::<String>(options::WIDTH) {
            None => 16,
            Some(s) => {
                if matches.value_source(options::WIDTH) == Some(ValueSource::CommandLine) {
                    match parse_number_of_bytes(s) {
                        Ok(n) => usize::try_from(n)
                            .map_err(|_| USimpleError::new(1, format!("‘{s}‘ is too large")))?,
                        Err(e) => {
                            return Err(USimpleError::new(
                                1,
                                format_error_message(&e, s, options::WIDTH),
                            ))
                        }
                    }
                } else {
                    16
                }
            }
        };

        let min_bytes = formats.iter().fold(1, |max, next| {
            cmp::max(max, next.formatter_item_info.byte_size)
        });
        if line_bytes == 0 || line_bytes % min_bytes != 0 {
            show_warning!("invalid width {}; using {} instead", line_bytes, min_bytes);
            line_bytes = min_bytes;
        }

        let output_duplicates = matches.get_flag(options::OUTPUT_DUPLICATES);

        let read_bytes = match matches.get_one::<String>(options::READ_BYTES) {
            None => None,
            Some(s) => match parse_number_of_bytes(s) {
                Ok(n) => Some(n),
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format_error_message(&e, s, options::READ_BYTES),
                    ))
                }
            },
        };

        let radix = match matches.get_one::<String>(options::ADDRESS_RADIX) {
            None => Radix::Octal,
            Some(s) => {
                let st = s.as_bytes();
                if st.len() == 1 {
                    let radix: char = *(st
                        .first()
                        .expect("byte string of length 1 lacks a 0th elem"))
                        as char;
                    match radix {
                        'd' => Radix::Decimal,
                        'x' => Radix::Hexadecimal,
                        'o' => Radix::Octal,
                        'n' => Radix::NoPrefix,
                        _ => {
                            return Err(USimpleError::new(
                                1,
                                "Radix must be one of [d, o, n, x]".to_string(),
                            ))
                        }
                    }
                } else {
                    return Err(USimpleError::new(
                        1,
                        "Radix must be one of [d, o, n, x]".to_string(),
                    ));
                }
            }
        };

        Ok(Self {
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
#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let clap_opts = uu_app();

    let clap_matches = clap_opts.try_get_matches_from(&args)?;

    let od_options = OdOptions::new(&clap_matches, &args)?;

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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .trailing_var_arg(true)
        .dont_delimit_trailing_values(true)
        .infer_long_args(true)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help)
        )
        .arg(
            Arg::new(options::ADDRESS_RADIX)
                .short('A')
                .long(options::ADDRESS_RADIX)
                .help("Select the base in which file offsets are printed.")
                .value_name("RADIX"),
        )
        .arg(
            Arg::new(options::SKIP_BYTES)
                .short('j')
                .long(options::SKIP_BYTES)
                .help("Skip bytes input bytes before formatting and writing.")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::READ_BYTES)
                .short('N')
                .long(options::READ_BYTES)
                .help("limit dump to BYTES input bytes")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::ENDIAN)
                .long(options::ENDIAN)
                .help("byte order to use for multi-byte formats")
                .value_parser(ShortcutValueParser::new(["big", "little"]))
                .value_name("big|little"),
        )
        .arg(
            Arg::new(options::STRINGS)
                .short('S')
                .long(options::STRINGS)
                .help(
                    "NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when \
                     BYTES is not specified.",
                )
                .default_missing_value("3")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new("a")
                .short('a')
                .help("named characters, ignoring high-order bit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("b")
                .short('b')
                .help("octal bytes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("c")
                .short('c')
                .help("ASCII characters or backslash escapes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("d")
                .short('d')
                .help("unsigned decimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("D")
                .short('D')
                .help("unsigned decimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .help("octal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("I")
                .short('I')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("L")
                .short('L')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .help("decimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("l")
                .short('l')
                .help("decimal 8-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("x")
                .short('x')
                .help("hexadecimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("h")
                .short('h')
                .help("hexadecimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("O")
                .short('O')
                .help("octal 4-byte units")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("s")
                .short('s')
                .help("decimal 2-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("X")
                .short('X')
                .help("hexadecimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("H")
                .short('H')
                .help("hexadecimal 4-byte units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .help("floating point double precision (64-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("f")
                .short('f')
                .help("floating point double precision (32-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("F")
                .short('F')
                .help("floating point double precision (64-bit) units")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('t')
                .long("format")
                .help("select output format or formats")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("TYPE"),
        )
        .arg(
            Arg::new(options::OUTPUT_DUPLICATES)
                .short('v')
                .long(options::OUTPUT_DUPLICATES)
                .help("do not use * to mark line suppression")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help(
                    "output BYTES bytes per output line. 32 is implied when BYTES is not \
                     specified.",
                )
                .default_missing_value("32")
                .value_name("BYTES")
                .num_args(..=1),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .long(options::TRADITIONAL)
                .help("compatibility mode with one input, offset and label.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILENAME)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

/// Loops through the input line by line, calling `print_bytes` to take care of the output.
fn odfunc<I>(
    input_offset: &mut InputOffset,
    input_decoder: &mut InputDecoder<I>,
    output_info: &OutputInfo,
) -> UResult<()>
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

                input_offset.increase_position(length as u64);
            }
            Err(e) => {
                show_error!("{}", e);
                input_offset.print_final_offset();
                return Err(1.into());
            }
        };
    }

    if input_decoder.has_error() {
        Err(1.into())
    } else {
        Ok(())
    }
}

/// Outputs a single line of input, into one or more lines human readable output.
fn print_bytes(prefix: &str, input_decoder: &MemoryDecoder, output_info: &OutputInfo) {
    let mut first = true; // First line of a multi-format raster.
    for f in output_info.spaced_formatters_iter() {
        let mut output_text = String::new();

        let mut b = 0;
        while b < input_decoder.length() {
            write!(
                output_text,
                "{:>width$}",
                "",
                width = f.spacing[b % output_info.byte_size_block]
            )
            .unwrap();

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
            write!(
                output_text,
                "{:>width$}  {}",
                "",
                format_ascii_dump(input_decoder.get_buffer(0)),
                width = missing_spacing
            )
            .unwrap();
        }

        if first {
            print!("{prefix}"); // print offset
                                // if printing in multiple formats offset is printed only once
            first = false;
        } else {
            // this takes the space of the file offset on subsequent
            // lines of multi-format rasters.
            print!("{:>width$}", "", width = prefix.chars().count());
        }
        println!("{output_text}");
    }
}

/// returns a reader implementing `PeekRead + Read + HasError` providing the combined input
///
/// `skip_bytes` is the number of bytes skipped from the input
/// `read_bytes` is an optional limit to the number of bytes to read
fn open_input_peek_reader(
    input_strings: &[String],
    skip_bytes: u64,
    read_bytes: Option<u64>,
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

fn format_error_message(error: &ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's od echos affected flag, -N or --read-bytes (-j or --skip-bytes, etc.), depending user's selection
    match error {
        ParseSizeError::InvalidSuffix(_) => {
            format!("invalid suffix in --{} argument {}", option, s.quote())
        }
        ParseSizeError::ParseFailure(_) => format!("invalid --{} argument {}", option, s.quote()),
        ParseSizeError::SizeTooBig(_) => format!("--{} argument {} too large", option, s.quote()),
    }
}
