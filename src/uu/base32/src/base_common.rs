// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore HEXUPPER Lsbf Msbf

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdout, Read, Write};
use std::io::{BufReader, Stdin};
use std::path::Path;
use uucore::display::Quotable;
use uucore::encoding::{
    for_fast_encode::{BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER},
    wrap_print, Data, EncodeError, Format,
};
use uucore::encoding::{BASE2LSBF, BASE2MSBF};
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;

pub const BASE_CMD_PARSE_ERROR: i32 = 1_i32;

/// Encoded output will be formatted in lines of this length (the last line can be shorter)
///
/// Other implementations default to 76
///
/// This default is only used if no "-w"/"--wrap" argument is passed
const WRAP_DEFAULT: usize = 76_usize;

pub struct Config {
    pub decode: bool,
    pub ignore_garbage: bool,
    pub wrap_cols: Option<usize>,
    pub to_read: Option<String>,
}

pub mod options {
    pub static DECODE: &str = "decode";
    pub static WRAP: &str = "wrap";
    pub static IGNORE_GARBAGE: &str = "ignore-garbage";
    pub static FILE: &str = "file";
}

impl Config {
    pub fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let file: Option<String> = match options.get_many::<String>(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();
                if let Some(extra_op) = values.next() {
                    return Err(UUsageError::new(
                        BASE_CMD_PARSE_ERROR,
                        format!("extra operand {}", extra_op.quote(),),
                    ));
                }

                if name == "-" {
                    None
                } else {
                    if !Path::exists(Path::new(name)) {
                        return Err(USimpleError::new(
                            BASE_CMD_PARSE_ERROR,
                            format!("{}: No such file or directory", name.maybe_quote()),
                        ));
                    }
                    Some(name.clone())
                }
            }
            None => None,
        };

        let cols = options
            .get_one::<String>(options::WRAP)
            .map(|num| {
                num.parse::<usize>().map_err(|_| {
                    USimpleError::new(
                        BASE_CMD_PARSE_ERROR,
                        format!("invalid wrap size: {}", num.quote()),
                    )
                })
            })
            .transpose()?;

        Ok(Self {
            decode: options.get_flag(options::DECODE),
            ignore_garbage: options.get_flag(options::IGNORE_GARBAGE),
            wrap_cols: cols,
            to_read: file,
        })
    }
}

pub fn parse_base_cmd_args(
    args: impl uucore::Args,
    about: &'static str,
    usage: &str,
) -> UResult<Config> {
    let command = base_app(about, usage);
    Config::from(&command.try_get_matches_from(args)?)
}

pub fn base_app(about: &'static str, usage: &str) -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(about)
        .override_usage(format_usage(usage))
        .infer_long_args(true)
        // Format arguments.
        .arg(
            Arg::new(options::DECODE)
                .short('d')
                .long(options::DECODE)
                .help("decode data")
                .action(ArgAction::SetTrue)
                .overrides_with(options::DECODE),
        )
        .arg(
            Arg::new(options::IGNORE_GARBAGE)
                .short('i')
                .long(options::IGNORE_GARBAGE)
                .help("when decoding, ignore non-alphabetic characters")
                .action(ArgAction::SetTrue)
                .overrides_with(options::IGNORE_GARBAGE),
        )
        .arg(
            Arg::new(options::WRAP)
                .short('w')
                .long(options::WRAP)
                .value_name("COLS")
                .help(format!("wrap encoded lines after COLS character (default {WRAP_DEFAULT}, 0 to disable wrapping)"))
                .overrides_with(options::WRAP),
        )
        // "multiple" arguments are used to check whether there is more than one
        // file passed in.
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

pub fn get_input<'a>(config: &Config, stdin_ref: &'a Stdin) -> UResult<Box<dyn Read + 'a>> {
    match &config.to_read {
        Some(name) => {
            let file_buf =
                File::open(Path::new(name)).map_err_context(|| name.maybe_quote().to_string())?;
            Ok(Box::new(BufReader::new(file_buf))) // as Box<dyn Read>
        }
        None => {
            Ok(Box::new(stdin_ref.lock())) // as Box<dyn Read>
        }
    }
}

pub fn handle_input<R: Read>(
    input: &mut R,
    format: Format,
    wrap: Option<usize>,
    ignore_garbage: bool,
    decode: bool,
) -> UResult<()> {
    const ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE: usize = 1_024_usize;

    // These constants indicate that inputs with lengths divisible by these numbers will have no padding characters
    // after encoding.
    // For instance:
    // "The quick brown"
    // is 15 characters (divisible by 3), so it is encoded in Base64 without padding:
    // "VGhlIHF1aWNrIGJyb3du"
    // While:
    // "The quick brown fox"
    // is 19 characters, which is not divisible by 3, so its Base64 representation has padding:
    // "VGhlIHF1aWNrIGJyb3duIGZveA=="
    // The encoding logic in this function depends on these constants being correct, so do not modify
    // them. Performance can be tuned by multiplying these numbers by a different multiple (see
    // `ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE` above).
    const BASE16_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE2_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE32_UN_PADDED_MULTIPLE: usize = 5_usize;
    const BASE64_UN_PADDED_MULTIPLE: usize = 3_usize;

    const BASE16_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE16_UN_PADDED_MULTIPLE * ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE2_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE2_UN_PADDED_MULTIPLE * ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE32_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE32_UN_PADDED_MULTIPLE * ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE64_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE64_UN_PADDED_MULTIPLE * ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;

    if decode {
        let mut data = Data::new(input, format);

        match data.decode(ignore_garbage) {
            Ok(s) => {
                // Silent the warning as we want to the error message
                #[allow(clippy::question_mark)]
                if stdout().write_all(&s).is_err() {
                    // on windows console, writing invalid utf8 returns an error
                    return Err(USimpleError::new(1, "error: cannot write non-utf8 data"));
                }
                Ok(())
            }
            Err(_) => Err(USimpleError::new(1, "error: invalid input")),
        }
    } else {
        #[allow(clippy::identity_op)]
        let encoding_and_encode_in_chunks_of_size = match format {
            // Use naive approach for Z85, since the crate being used doesn't have the API needed
            Format::Z85 => {
                let mut data = Data::new(input, format);

                let result = match data.encode() {
                    Ok(st) => {
                        wrap_print(&st, wrap.unwrap_or(WRAP_DEFAULT))?;

                        Ok(())
                    }
                    Err(EncodeError::InvalidInput) => {
                        Err(USimpleError::new(1, "error: invalid input"))
                    }
                    Err(_) => Err(USimpleError::new(
                        1,
                        "error: invalid input (length must be multiple of 4 characters)",
                    )),
                };

                return result;
            }

            // For these, use faster, new encoding logic
            Format::Base16 => (HEXUPPER, BASE16_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base2Lsbf => (BASE2LSBF, BASE2_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base2Msbf => (BASE2MSBF, BASE2_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base32 => (BASE32, BASE32_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base32Hex => (BASE32HEX, BASE32_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base64 => (BASE64, BASE64_ENCODE_IN_CHUNKS_OF_SIZE),
            Format::Base64Url => (BASE64URL, BASE64_ENCODE_IN_CHUNKS_OF_SIZE),
        };

        fast_encode::fast_encode(input, encoding_and_encode_in_chunks_of_size, wrap)?;

        Ok(())
    }
}

mod fast_encode {
    use crate::base_common::WRAP_DEFAULT;
    use std::{
        collections::VecDeque,
        io::{self, ErrorKind, Read, StdoutLock, Write},
    };
    use uucore::{
        encoding::for_fast_encode::Encoding,
        error::{UResult, USimpleError},
    };

    struct LineWrapping {
        line_length: usize,
        print_buffer: Vec<u8>,
    }

    // Start of helper functions
    // Adapted from `encode_append` in the "data-encoding" crate
    fn encode_append_vec_deque(encoding: &Encoding, input: &[u8], output: &mut VecDeque<u8>) {
        let output_len = output.len();

        output.resize(output_len + encoding.encode_len(input.len()), 0_u8);

        let make_contiguous_result = output.make_contiguous();

        encoding.encode_mut(input, &mut (make_contiguous_result[output_len..]));
    }

    fn write_without_line_breaks(
        encoded_buffer: &mut VecDeque<u8>,
        stdout_lock: &mut StdoutLock,
        is_cleanup: bool,
    ) -> io::Result<()> {
        // TODO
        // `encoded_buffer` only has to be a VecDeque if line wrapping is enabled
        // (`make_contiguous` should be a no-op here)
        // Refactoring could avoid this call
        stdout_lock.write_all(encoded_buffer.make_contiguous())?;

        if is_cleanup {
            stdout_lock.write_all(b"\n")?;
        } else {
            encoded_buffer.truncate(0_usize);
        }

        Ok(())
    }

    fn write_with_line_breaks(
        &mut LineWrapping {
            ref line_length,
            ref mut print_buffer,
        }: &mut LineWrapping,
        encoded_buffer: &mut VecDeque<u8>,
        stdout_lock: &mut StdoutLock,
        is_cleanup: bool,
    ) -> io::Result<()> {
        let line_length_usize = *line_length;

        assert!(line_length_usize > 0_usize);

        let number_of_lines = encoded_buffer.len() / line_length_usize;

        // How many bytes to take from the front of `encoded_buffer` and then write to stdout
        let number_of_bytes_to_drain = number_of_lines * line_length_usize;

        let line_wrap_size_minus_one = line_length_usize - 1_usize;

        let mut i = 0_usize;

        for ue in encoded_buffer.drain(0_usize..number_of_bytes_to_drain) {
            print_buffer.push(ue);

            if i == line_wrap_size_minus_one {
                print_buffer.push(b'\n');

                i = 0_usize;
            } else {
                i += 1_usize;
            }
        }

        stdout_lock.write_all(print_buffer)?;

        if is_cleanup {
            if encoded_buffer.is_empty() {
                // Do not write a newline in this case, because two trailing newlines should never be printed
            } else {
                // Print the partial line, since this is cleanup and no more data is coming
                stdout_lock.write_all(encoded_buffer.make_contiguous())?;
                stdout_lock.write_all(b"\n")?;
            }
        } else {
            print_buffer.truncate(0_usize);
        }

        Ok(())
    }

    fn write_to_stdout(
        line_wrapping_option: &mut Option<LineWrapping>,
        encoded_buffer: &mut VecDeque<u8>,
        stdout_lock: &mut StdoutLock,
        is_cleanup: bool,
    ) -> io::Result<()> {
        // Write all data in `encoded_buffer` to stdout
        if let &mut Some(ref mut li) = line_wrapping_option {
            write_with_line_breaks(li, encoded_buffer, stdout_lock, is_cleanup)?;
        } else {
            write_without_line_breaks(encoded_buffer, stdout_lock, is_cleanup)?;
        }

        Ok(())
    }
    // End of helper functions

    // TODO
    // It turns out the crate being used already supports line wrapping:
    // https://docs.rs/data-encoding/latest/data_encoding/struct.Specification.html#wrap-output-when-encoding-1
    // Check if that crate's line wrapping is faster than the wrapping being performed in this function
    // Update: That crate does not support arbitrary width line wrapping. It only supports certain widths:
    // https://github.com/ia0/data-encoding/blob/4f42ad7ef242f6d243e4de90cd1b46a57690d00e/lib/src/lib.rs#L1710
    // `encoding` and `encode_in_chunks_of_size` are passed in a tuple to indicate that they are logically tied
    pub fn fast_encode<R: Read>(
        input: &mut R,
        (encoding, encode_in_chunks_of_size): (Encoding, usize),
        line_wrap: Option<usize>,
    ) -> UResult<()> {
        /// Rust uses 8 kibibytes
        ///
        /// https://github.com/rust-lang/rust/blob/1a5a2240bc1b8cf0bcce7acb946c78d6493a4fd3/library/std/src/sys_common/io.rs#L3
        const INPUT_BUFFER_SIZE: usize = 8_usize * 1_024_usize;

        let mut line_wrapping_option = match line_wrap {
            // Line wrapping is disabled because "-w"/"--wrap" was passed with "0"
            Some(0_usize) => None,
            // A custom line wrapping value was passed
            Some(an) => Some(LineWrapping {
                line_length: an,
                print_buffer: Vec::<u8>::new(),
            }),
            // Line wrapping was not set, so the default is used
            None => Some(LineWrapping {
                line_length: WRAP_DEFAULT,
                print_buffer: Vec::<u8>::new(),
            }),
        };

        // Start of buffers
        // Data that was read from stdin
        let mut input_buffer = vec![0_u8; INPUT_BUFFER_SIZE];

        assert!(!input_buffer.is_empty());

        // Data that was read from stdin but has not been encoded yet
        let mut leftover_buffer = VecDeque::<u8>::new();

        // Encoded data that needs to be written to stdout
        let mut encoded_buffer = VecDeque::<u8>::new();
        // End of buffers

        let mut stdout_lock = io::stdout().lock();

        loop {
            match input.read(&mut input_buffer) {
                Ok(bytes_read_from_input) => {
                    if bytes_read_from_input == 0_usize {
                        break;
                    }

                    // The part of `input_buffer` that was actually filled by the call to `read`
                    let read_buffer = &input_buffer[0_usize..bytes_read_from_input];

                    // How many bytes to steal from `read_buffer` to get `leftover_buffer` to the right size
                    let bytes_to_steal = encode_in_chunks_of_size - leftover_buffer.len();

                    if bytes_to_steal > bytes_read_from_input {
                        // Do not have enough data to encode a chunk, so copy data to `leftover_buffer` and read more
                        leftover_buffer.extend(read_buffer);

                        continue;
                    }

                    // Encode data in chunks, then place it in `encoded_buffer`
                    {
                        let bytes_to_chunk = if bytes_to_steal > 0 {
                            let (stolen_bytes, rest_of_read_buffer) =
                                read_buffer.split_at(bytes_to_steal);

                            leftover_buffer.extend(stolen_bytes);

                            // After appending the stolen bytes to `leftover_buffer`, it should be the right size
                            assert!(leftover_buffer.len() == encode_in_chunks_of_size);

                            // Encode the old unencoded data and the stolen bytes, and add the result to
                            // `encoded_buffer`
                            encode_append_vec_deque(
                                &encoding,
                                leftover_buffer.make_contiguous(),
                                &mut encoded_buffer,
                            );

                            // Reset `leftover_buffer`
                            leftover_buffer.truncate(0_usize);

                            rest_of_read_buffer
                        } else {
                            // Do not need to steal bytes from `read_buffer`
                            read_buffer
                        };

                        let chunks_exact = bytes_to_chunk.chunks_exact(encode_in_chunks_of_size);

                        let remainder = chunks_exact.remainder();

                        for sl in chunks_exact {
                            assert!(sl.len() == encode_in_chunks_of_size);

                            encode_append_vec_deque(&encoding, sl, &mut encoded_buffer);
                        }

                        leftover_buffer.extend(remainder);
                    }

                    // Write all data in `encoded_buffer` to stdout
                    write_to_stdout(
                        &mut line_wrapping_option,
                        &mut encoded_buffer,
                        &mut stdout_lock,
                        false,
                    )?;
                }
                Err(er) => {
                    if er.kind() == ErrorKind::Interrupted {
                        // TODO
                        // Retry reading?
                    }

                    return Err(USimpleError::new(1_i32, format!("read error: {er}")));
                }
            }
        }

        // Cleanup
        // `input` has finished producing data, so the data remaining in the buffers needs to be encoded and printed
        {
            // Encode all remaining unencoded bytes, placing it in `encoded_buffer`
            encode_append_vec_deque(
                &encoding,
                leftover_buffer.make_contiguous(),
                &mut encoded_buffer,
            );

            // Write all data in `encoded_buffer` to stdout
            // `is_cleanup` triggers special cleanup-only logic
            write_to_stdout(
                &mut line_wrapping_option,
                &mut encoded_buffer,
                &mut stdout_lock,
                true,
            )?;
        }

        Ok(())
    }
}
