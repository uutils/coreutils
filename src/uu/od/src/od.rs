// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (clap) dont
// spell-checker:ignore (ToDO) formatteriteminfo inputdecoder inputoffset mockstream nrofbytes partialreader odfunc multifile exitcode
// spell-checker:ignore Anone bfloat

mod byteorder_io;
mod formatter_item_info;
mod input_decoder;
mod input_offset;
#[cfg(test)]
mod mockstream;
mod multifile_reader;
mod output_info;
mod parse_formats;
mod parse_inputs;
mod parse_nrofbytes;
mod partial_reader;
mod peek_reader;
mod prn_char;
mod prn_float;
mod prn_int;

use std::cmp;
use std::fmt::Write;
use std::io::{BufReader, Read};

use crate::byteorder_io::ByteOrder;
use crate::formatter_item_info::FormatWriter;
use crate::input_decoder::{InputDecoder, MemoryDecoder};
use crate::input_offset::{InputOffset, Radix};
use crate::multifile_reader::{HasError, InputSource, MultifileReader};
use crate::output_info::OutputInfo;
use crate::parse_formats::{ParsedFormatterItemInfo, parse_format_flags};
use crate::parse_inputs::{CommandLineInputs, parse_inputs};
use crate::parse_nrofbytes::parse_number_of_bytes;
use crate::partial_reader::PartialReader;
use crate::peek_reader::{PeekRead, PeekReader};
use crate::prn_char::format_ascii_dump;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command, parser::ValueSource};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::translate;

use uucore::parser::parse_size::ParseSizeError;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, show_error, show_warning};

const PEEK_BUFFER_SIZE: usize = 4; // utf-8 can be 4 bytes

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
    string_min_length: Option<usize>,
}

/// Helper function to parse bytes with error handling
fn parse_bytes_option(matches: &ArgMatches, option_name: &str) -> UResult<Option<u64>> {
    match matches.get_one::<String>(option_name) {
        None => Ok(None),
        Some(s) => match parse_number_of_bytes(s) {
            Ok(n) => Ok(Some(n)),
            Err(e) => Err(USimpleError::new(
                1,
                format_error_message(&e, s, option_name),
            )),
        },
    }
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
                        translate!("od-error-invalid-endian", "endian" => s),
                    ));
                }
            }
        } else {
            ByteOrder::Native
        };

        let mut skip_bytes = parse_bytes_option(matches, options::SKIP_BYTES)?.unwrap_or(0);

        let mut label: Option<u64> = None;

        let parsed_input = parse_inputs(matches)
            .map_err(|e| USimpleError::new(1, translate!("od-error-invalid-inputs", "msg" => e)))?;
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
                            ));
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
            show_warning!(
                "{}",
                translate!("od-error-invalid-width", "width" => line_bytes, "min" => min_bytes)
            );
            line_bytes = min_bytes;
        }

        let output_duplicates = matches.get_flag(options::OUTPUT_DUPLICATES);

        let read_bytes = parse_bytes_option(matches, options::READ_BYTES)?;

        let string_min_length = match parse_bytes_option(matches, options::STRINGS)? {
            None => None,
            Some(n) => Some(usize::try_from(n).map_err(|_| {
                USimpleError::new(
                    1,
                    translate!("od-error-argument-too-large", "option" => "-S", "value" => n.to_string()),
                )
            })?),
        };

        let radix = match matches.get_one::<String>(options::ADDRESS_RADIX) {
            None => Radix::Octal,
            Some(s) => {
                // Other implementations of od only check the first character of this argument's value.
                // This means executing `od -Anone` is equivalent to executing `od -An`.
                // Existing users of od rely on this behavior:
                // https://github.com/landley/toybox/blob/d50372cad35d5dd12e6391c3c7c901a96122dc67/scripts/make.sh#L239
                // https://github.com/google/jsonnet/blob/913281d203578bb394995bacc792f2576371e06c/Makefile#L212
                let st = s.as_bytes();
                if let Some(u) = st.first() {
                    match *u {
                        b'o' => Radix::Octal,
                        b'd' => Radix::Decimal,
                        b'x' => Radix::Hexadecimal,
                        b'n' => Radix::NoPrefix,
                        _ => {
                            return Err(USimpleError::new(
                                1,
                                translate!("od-error-radix-invalid", "radix" => s),
                            ));
                        }
                    }
                } else {
                    // Return an error instead of panicking when `od -A ''` is executed.
                    return Err(USimpleError::new(1, translate!("od-error-radix-empty")));
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
            string_min_length,
        })
    }
}

/// parses and validates command line parameters, prepares data structures,
/// opens the input and calls `odfunc` to process the input.
#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let clap_opts = uu_app();

    let clap_matches = uucore::clap_localization::handle_clap_result(clap_opts, &args)?;

    let od_options = OdOptions::new(&clap_matches, &args)?;

    // Check if we're in strings mode
    if let Some(min_length) = od_options.string_min_length {
        extract_strings_from_input(
            &od_options.input_strings,
            od_options.skip_bytes,
            od_options.read_bytes,
            min_length,
            od_options.radix,
        )
    } else {
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
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("od-about"))
        .override_usage(format_usage(&translate!("od-usage")))
        .after_help(translate!("od-after-help"))
        .dont_delimit_trailing_values(true)
        .infer_long_args(true)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("od-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::ADDRESS_RADIX)
                .short('A')
                .long(options::ADDRESS_RADIX)
                .help(translate!("od-help-address-radix"))
                .value_name("RADIX"),
        )
        .arg(
            Arg::new(options::SKIP_BYTES)
                .short('j')
                .long(options::SKIP_BYTES)
                .help(translate!("od-help-skip-bytes"))
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::READ_BYTES)
                .short('N')
                .long(options::READ_BYTES)
                .help(translate!("od-help-read-bytes"))
                .value_name("BYTES"),
        )
        .arg(
            Arg::new(options::ENDIAN)
                .long(options::ENDIAN)
                .help(translate!("od-help-endian"))
                .value_parser(ShortcutValueParser::new(["big", "little"]))
                .value_name("big|little"),
        )
        .arg(
            Arg::new(options::STRINGS)
                .short('S')
                .long(options::STRINGS)
                .help(translate!("od-help-strings"))
                .num_args(0..=1)
                .default_missing_value("3")
                .value_name("BYTES"),
        )
        .arg(
            Arg::new("a")
                .short('a')
                .help(translate!("od-help-a"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("b")
                .short('b')
                .help(translate!("od-help-b"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("c")
                .short('c')
                .help(translate!("od-help-c"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("d")
                .short('d')
                .help(translate!("od-help-d"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("D")
                .short('D')
                .help(translate!("od-help-d4"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .help(translate!("od-help-o"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("I")
                .short('I')
                .help(translate!("od-help-capital-i"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("L")
                .short('L')
                .help(translate!("od-help-capital-l"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .help(translate!("od-help-i"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("l")
                .short('l')
                .help(translate!("od-help-l"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("x")
                .short('x')
                .help(translate!("od-help-x"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("h")
                .short('h')
                .help(translate!("od-help-h"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("O")
                .short('O')
                .help(translate!("od-help-capital-o"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("s")
                .short('s')
                .help(translate!("od-help-s"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("X")
                .short('X')
                .help(translate!("od-help-capital-x"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("H")
                .short('H')
                .help(translate!("od-help-capital-h"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .help(translate!("od-help-e"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("f")
                .short('f')
                .help(translate!("od-help-f"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("F")
                .short('F')
                .help(translate!("od-help-capital-f"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('t')
                .long("format")
                .help(translate!("od-help-format"))
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("TYPE"),
        )
        .arg(
            Arg::new(options::OUTPUT_DUPLICATES)
                .short('v')
                .long(options::OUTPUT_DUPLICATES)
                .help(translate!("od-help-output-duplicates"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long(options::WIDTH)
                .help(translate!("od-help-width"))
                .default_missing_value("32")
                .value_name("BYTES")
                .num_args(..=1),
        )
        .arg(
            Arg::new(options::TRADITIONAL)
                .long(options::TRADITIONAL)
                .help(translate!("od-help-traditional"))
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
                show_error!("{e}");
                input_offset.print_final_offset();
                return Err(1.into());
            }
        }
    }

    if input_decoder.has_error() {
        Err(1.into())
    } else {
        Ok(())
    }
}

/// Extract and display printable strings from input (od -S option)
fn extract_strings_from_input(
    input_strings: &[String],
    skip_bytes: u64,
    read_bytes: Option<u64>,
    min_length: usize,
    radix: Radix,
) -> UResult<()> {
    let inputs = map_input_strings(input_strings);
    let mut mf = MultifileReader::new(inputs);

    // Apply skip_bytes by reading and discarding
    let mut skipped = 0u64;
    while skipped < skip_bytes {
        let to_skip = std::cmp::min(8192, skip_bytes - skipped);
        let mut skip_buf = vec![0u8; to_skip as usize];
        match mf.read(&mut skip_buf) {
            Ok(0) => break, // EOF reached
            Ok(n) => skipped += n as u64,
            Err(_) => break,
        }
    }

    // Helper function to format and print a string
    let print_string = |offset: u64, string: &[u8]| {
        let string_content = String::from_utf8_lossy(string);
        match radix {
            Radix::NoPrefix => println!("{string_content}"),
            Radix::Decimal => println!("{offset:07} {string_content}"),
            Radix::Hexadecimal => println!("{offset:07x} {string_content}"),
            Radix::Octal => println!("{offset:07o} {string_content}"),
        }
    };

    let mut current_string = Vec::new();
    let mut string_start_offset = 0u64;
    let mut current_offset = skip_bytes;
    let mut bytes_read = 0u64;
    let mut buf = [0u8; 1];

    loop {
        // Check if we've reached the read_bytes limit
        if let Some(limit) = read_bytes {
            if bytes_read >= limit {
                // Special case: when -N limit is reached with a pending string
                // that meets min_length, output it even without null terminator
                if current_string.len() >= min_length {
                    print_string(string_start_offset, &current_string);
                }
                break;
            }
        }

        // Read one byte at a time
        match mf.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                bytes_read += 1;
                let byte = buf[0];

                // Check if it's a printable character (including space)
                if (0x20..=0x7E).contains(&byte) {
                    if current_string.is_empty() {
                        string_start_offset = current_offset;
                    }
                    current_string.push(byte);
                } else {
                    // Either null terminator or non-printable character
                    if byte == 0 && current_string.len() >= min_length {
                        // Null terminator found with valid string
                        print_string(string_start_offset, &current_string);
                    }
                    current_string.clear();
                }

                current_offset += 1;
            }
            Err(e) => {
                // Note: GNU od does not output unterminated strings at EOF
                // Strings must be null-terminated to be output
                if mf.has_error() {
                    show_error!("{}", e);
                    return Err(1.into());
                }
                break;
            }
        }
    }

    // GNU od doesn't output an offset when strings mode finds no valid strings
    // This includes cases with only unterminated or too-short strings

    if mf.has_error() {
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
                FormatWriter::BFloatWriter(func) => {
                    let p = input_decoder.read_bfloat(b);
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
                "{:>missing_spacing$}  {}",
                "",
                format_ascii_dump(input_decoder.get_buffer(0)),
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

/// Helper function to convert input strings to InputSource
fn map_input_strings(input_strings: &[String]) -> Vec<InputSource<'_>> {
    input_strings
        .iter()
        .map(|w| match w as &str {
            "-" => InputSource::Stdin,
            x => InputSource::FileName(x),
        })
        .collect()
}

/// returns a reader implementing `PeekRead + Read + HasError` providing the combined input
///
/// `skip_bytes` is the number of bytes skipped from the input
/// `read_bytes` is an optional limit to the number of bytes to read
fn open_input_peek_reader(
    input_strings: &[String],
    skip_bytes: u64,
    read_bytes: Option<u64>,
) -> PeekReader<BufReader<PartialReader<MultifileReader<'_>>>> {
    // should return  "impl PeekRead + Read + HasError" when supported in (stable) rust
    let inputs = map_input_strings(input_strings);
    let mf = MultifileReader::new(inputs);
    let pr = PartialReader::new(mf, skip_bytes, read_bytes);
    // Add a BufReader over the top of the PartialReader. This will have the
    // effect of generating buffered reads to files/stdin, but since these reads
    // go through MultifileReader (which limits the maximum number of bytes read)
    // we won't ever read more bytes than were specified with the `-N` flag.
    let buf_pr = BufReader::new(pr);
    PeekReader::new(buf_pr)
}

impl<R: HasError> HasError for BufReader<R> {
    fn has_error(&self) -> bool {
        self.get_ref().has_error()
    }
}

fn format_error_message(error: &ParseSizeError, s: &str, option: &str) -> String {
    // NOTE:
    // GNU's od echos affected flag, -N or --read-bytes (-j or --skip-bytes, etc.), depending user's selection
    match error {
        ParseSizeError::InvalidSuffix(_) => {
            translate!("od-error-invalid-suffix", "option" => option, "value" => s.quote())
        }
        ParseSizeError::ParseFailure(_) | ParseSizeError::PhysicalMem(_) => {
            translate!("od-error-invalid-argument", "option" => option, "value" => s.quote())
        }
        ParseSizeError::SizeTooBig(_) => {
            translate!("od-error-argument-too-large", "option" => option, "value" => s.quote())
        }
    }
}
