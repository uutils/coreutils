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
use uucore::encoding::{decode_z_eight_five, encode_z_eight_five, BASE2LSBF, BASE2MSBF};
use uucore::encoding::{
    for_fast_encode::{BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER},
    wrap_print, EncodeError, Format,
};
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
    const DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE: usize = 1_024_usize;

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
    // `DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE` above).
    const BASE16_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE2_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE32_UN_PADDED_MULTIPLE: usize = 5_usize;
    const BASE64_UN_PADDED_MULTIPLE: usize = 3_usize;

    const BASE16_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE16_UN_PADDED_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE2_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE2_UN_PADDED_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE32_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE32_UN_PADDED_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE64_ENCODE_IN_CHUNKS_OF_SIZE: usize =
        BASE64_UN_PADDED_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;

    const BASE16_VALID_DECODING_MULTIPLE: usize = 2_usize;
    const BASE2_VALID_DECODING_MULTIPLE: usize = 8_usize;
    const BASE32_VALID_DECODING_MULTIPLE: usize = 8_usize;
    const BASE64_VALID_DECODING_MULTIPLE: usize = 4_usize;

    const BASE16_DECODE_IN_CHUNKS_OF_SIZE: usize =
        BASE16_VALID_DECODING_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE2_DECODE_IN_CHUNKS_OF_SIZE: usize =
        BASE2_VALID_DECODING_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE32_DECODE_IN_CHUNKS_OF_SIZE: usize =
        BASE32_VALID_DECODING_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
    const BASE64_DECODE_IN_CHUNKS_OF_SIZE: usize =
        BASE64_VALID_DECODING_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;

    if decode {
        let encoding_and_decode_in_chunks_of_size_and_alphabet: (_, _, &[u8]) = match format {
            // Use naive approach for Z85, since the crate being used doesn't have the API needed
            Format::Z85 => {
                let result = match decode_z_eight_five(input, ignore_garbage) {
                    Ok(ve) => {
                        if stdout().write_all(&ve).is_err() {
                            // on windows console, writing invalid utf8 returns an error
                            return Err(USimpleError::new(
                                1_i32,
                                "error: cannot write non-utf8 data",
                            ));
                        }

                        Ok(())
                    }
                    Err(_) => Err(USimpleError::new(1_i32, "error: invalid input")),
                };

                return result;
            }

            // For these, use faster, new decoding logic
            Format::Base16 => (
                HEXUPPER,
                BASE16_DECODE_IN_CHUNKS_OF_SIZE,
                b"0123456789ABCDEF",
            ),
            Format::Base2Lsbf => (BASE2LSBF, BASE2_DECODE_IN_CHUNKS_OF_SIZE, b"01"),
            Format::Base2Msbf => (BASE2MSBF, BASE2_DECODE_IN_CHUNKS_OF_SIZE, b"01"),
            Format::Base32 => (
                BASE32,
                BASE32_DECODE_IN_CHUNKS_OF_SIZE,
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
            ),
            Format::Base32Hex => (
                BASE32HEX,
                BASE32_DECODE_IN_CHUNKS_OF_SIZE,
                // spell-checker:disable-next-line
                b"0123456789ABCDEFGHIJKLMNOPQRSTUV=",
            ),
            Format::Base64 => (
                BASE64,
                BASE64_DECODE_IN_CHUNKS_OF_SIZE,
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=+/",
            ),
            Format::Base64Url => (
                BASE64URL,
                BASE64_DECODE_IN_CHUNKS_OF_SIZE,
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=_-",
            ),
        };

        fast_decode::fast_decode(
            input,
            encoding_and_decode_in_chunks_of_size_and_alphabet,
            ignore_garbage,
        )?;

        Ok(())
    } else {
        let encoding_and_encode_in_chunks_of_size = match format {
            // Use naive approach for Z85, since the crate being used doesn't have the API needed
            Format::Z85 => {
                let result = match encode_z_eight_five(input) {
                    Ok(st) => {
                        wrap_print(&st, wrap.unwrap_or(WRAP_DEFAULT))?;

                        Ok(())
                    }
                    Err(EncodeError::InvalidInput) => {
                        Err(USimpleError::new(1_i32, "error: invalid input"))
                    }
                    Err(_) => Err(USimpleError::new(
                        1_i32,
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
            encoded_buffer.clear();
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

        let make_contiguous_result = encoded_buffer.make_contiguous();

        // How many bytes to take from the front of `encoded_buffer` and then write to stdout
        // (Number of whole lines times the line length)
        let number_of_bytes_to_drain =
            (make_contiguous_result.len() / line_length_usize) * line_length_usize;

        let chunks_exact = make_contiguous_result.chunks_exact(line_length_usize);

        for sl in chunks_exact {
            print_buffer.extend_from_slice(sl);
            print_buffer.push(b'\n');
        }

        stdout_lock.write_all(print_buffer)?;

        // Remove the bytes that were just printed from `encoded_buffer`
        drop(encoded_buffer.drain(..number_of_bytes_to_drain));

        if is_cleanup {
            if encoded_buffer.is_empty() {
                // Do not write a newline in this case, because two trailing newlines should never be printed
            } else {
                // Print the partial line, since this is cleanup and no more data is coming
                stdout_lock.write_all(encoded_buffer.make_contiguous())?;
                stdout_lock.write_all(b"\n")?;
            }
        } else {
            print_buffer.clear();
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
    //
    /// `encoding` and `encode_in_chunks_of_size` are passed in a tuple to indicate that they are logically tied
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
                    let read_buffer = &input_buffer[..bytes_read_from_input];

                    // How many bytes to steal from `read_buffer` to get `leftover_buffer` to the right size
                    let bytes_to_steal = encode_in_chunks_of_size - leftover_buffer.len();

                    if bytes_to_steal > bytes_read_from_input {
                        // Do not have enough data to encode a chunk, so copy data to `leftover_buffer` and read more
                        leftover_buffer.extend(read_buffer);

                        continue;
                    }

                    // Encode data in chunks, then place it in `encoded_buffer`
                    {
                        let bytes_to_chunk = if bytes_to_steal > 0_usize {
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
                            leftover_buffer.clear();

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
            // Encode all remaining unencoded bytes, placing them in `encoded_buffer`
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

mod fast_decode {
    use std::io::{self, ErrorKind, Read, StdoutLock, Write};
    use uucore::{
        encoding::{alphabet_to_table, for_fast_encode::Encoding},
        error::{UResult, USimpleError},
    };

    struct FilteringData {
        table: [bool; 256_usize],
    }

    // Start of helper functions
    // Adapted from `decode` in the "data-encoding" crate
    fn decode_into_vec(encoding: &Encoding, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        let decode_len_result = match encoding.decode_len(input.len()) {
            Ok(us) => us,
            Err(de) => {
                return Err(USimpleError::new(1_i32, format!("{de}")));
            }
        };

        let output_len = output.len();

        output.resize(output_len + decode_len_result, 0_u8);

        match encoding.decode_mut(input, &mut (output[output_len..])) {
            Ok(us) => {
                // See:
                // https://docs.rs/data-encoding/latest/data_encoding/struct.Encoding.html#method.decode_mut
                // "Returns the length of the decoded output. This length may be smaller than the output length if the input contained padding or ignored characters. The output bytes after the returned length are not initialized and should not be read."
                output.truncate(output_len + us);
            }
            Err(_de) => {
                return Err(USimpleError::new(1_i32, "error: invalid input".to_owned()));
            }
        }

        Ok(())
    }

    fn write_to_stdout(
        decoded_buffer: &mut Vec<u8>,
        stdout_lock: &mut StdoutLock,
    ) -> io::Result<()> {
        // Write all data in `decoded_buffer` to stdout
        stdout_lock.write_all(decoded_buffer.as_slice())?;

        decoded_buffer.clear();

        Ok(())
    }
    // End of helper functions

    /// `encoding`, `decode_in_chunks_of_size`, and `alphabet` are passed in a tuple to indicate that they are
    /// logically tied
    pub fn fast_decode<R: Read>(
        input: &mut R,
        (encoding, decode_in_chunks_of_size, alphabet): (Encoding, usize, &[u8]),
        ignore_garbage: bool,
    ) -> UResult<()> {
        /// Rust uses 8 kibibytes
        ///
        /// https://github.com/rust-lang/rust/blob/1a5a2240bc1b8cf0bcce7acb946c78d6493a4fd3/library/std/src/sys_common/io.rs#L3
        const INPUT_BUFFER_SIZE: usize = 8_usize * 1_024_usize;

        // Note that it's not worth using "data-encoding"'s ignore functionality if "ignore_garbage" is true, because
        // "data-encoding"'s ignore functionality cannot discard non-ASCII bytes. The data has to be filtered before
        // passing it to "data-encoding", so there is no point in doing any filtering in "data-encoding". This also
        // allows execution to stay on the happy path in "data-encoding":
        // https://github.com/ia0/data-encoding/blob/4f42ad7ef242f6d243e4de90cd1b46a57690d00e/lib/src/lib.rs#L754-L756
        let (encoding_to_use, filter_data_option) = {
            if ignore_garbage {
                // Note that the alphabet constants above already include the padding characters
                // TODO
                // Precompute this
                let table = alphabet_to_table(alphabet);

                (encoding, Some(FilteringData { table }))
            } else {
                let mut sp = encoding.specification();

                // '\n' and '\r' are always ignored
                sp.ignore = "\n\r".to_owned();

                let en = match sp.encoding() {
                    Ok(en) => en,
                    Err(sp) => {
                        return Err(USimpleError::new(1_i32, format!("{sp}")));
                    }
                };

                (en, None)
            }
        };

        // Start of buffers
        // Data that was read from stdin
        let mut input_buffer = vec![0_u8; INPUT_BUFFER_SIZE];

        assert!(!input_buffer.is_empty());

        // Data that was read from stdin but has not been decoded yet
        let mut leftover_buffer = Vec::<u8>::new();

        // Decoded data that needs to be written to stdout
        let mut decoded_buffer = Vec::<u8>::new();

        // Buffer that will be used when "ignore_garbage" is true, and the chunk read from "input" contains garbage
        // data
        let mut non_garbage_buffer = Vec::<u8>::new();
        // End of buffers

        let mut stdout_lock = io::stdout().lock();

        loop {
            match input.read(&mut input_buffer) {
                Ok(bytes_read_from_input) => {
                    if bytes_read_from_input == 0_usize {
                        break;
                    }

                    let read_buffer_filtered = {
                        // The part of `input_buffer` that was actually filled by the call to `read`
                        let read_buffer = &input_buffer[..bytes_read_from_input];

                        if let Some(fi) = &filter_data_option {
                            let FilteringData { table } = fi;

                            let table_to_owned = table.to_owned();

                            // First just scan the data for the happy path
                            // Note: this happy path check has not been validated with performance testing
                            let mut found_garbage = false;

                            for ue in read_buffer {
                                if table_to_owned[usize::from(*ue)] {
                                    // Not garbage, since it was found in the table
                                    continue;
                                } else {
                                    found_garbage = true;

                                    break;
                                }
                            }

                            if found_garbage {
                                non_garbage_buffer.clear();

                                for ue in read_buffer {
                                    if table_to_owned[usize::from(*ue)] {
                                        // Not garbage, since it was found in the table
                                        non_garbage_buffer.push(*ue);
                                    }
                                }

                                non_garbage_buffer.as_slice()
                            } else {
                                read_buffer
                            }
                        } else {
                            read_buffer
                        }
                    };

                    // How many bytes to steal from `read_buffer` to get `leftover_buffer` to the right size
                    let bytes_to_steal = decode_in_chunks_of_size - leftover_buffer.len();

                    if bytes_to_steal > bytes_read_from_input {
                        // Do not have enough data to decode a chunk, so copy data to `leftover_buffer` and read more
                        leftover_buffer.extend(read_buffer_filtered);

                        continue;
                    }

                    // Decode data in chunks, then place it in `decoded_buffer`
                    {
                        let bytes_to_chunk = if bytes_to_steal > 0_usize {
                            let (stolen_bytes, rest_of_read_buffer_filtered) =
                                read_buffer_filtered.split_at(bytes_to_steal);

                            leftover_buffer.extend(stolen_bytes);

                            // After appending the stolen bytes to `leftover_buffer`, it should be the right size
                            assert!(leftover_buffer.len() == decode_in_chunks_of_size);

                            // Decode the old un-decoded data and the stolen bytes, and add the result to
                            // `decoded_buffer`
                            decode_into_vec(
                                &encoding_to_use,
                                &leftover_buffer,
                                &mut decoded_buffer,
                            )?;

                            // Reset `leftover_buffer`
                            leftover_buffer.clear();

                            rest_of_read_buffer_filtered
                        } else {
                            // Do not need to steal bytes from `read_buffer`
                            read_buffer_filtered
                        };

                        let chunks_exact = bytes_to_chunk.chunks_exact(decode_in_chunks_of_size);

                        let remainder = chunks_exact.remainder();

                        for sl in chunks_exact {
                            assert!(sl.len() == decode_in_chunks_of_size);

                            decode_into_vec(&encoding_to_use, sl, &mut decoded_buffer)?;
                        }

                        leftover_buffer.extend(remainder);
                    }

                    // Write all data in `decoded_buffer` to stdout
                    write_to_stdout(&mut decoded_buffer, &mut stdout_lock)?;
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
        // `input` has finished producing data, so the data remaining in the buffers needs to be decoded and printed
        {
            // Decode all remaining encoded bytes, placing them in `decoded_buffer`
            decode_into_vec(&encoding_to_use, &leftover_buffer, &mut decoded_buffer)?;

            // Write all data in `decoded_buffer` to stdout
            write_to_stdout(&mut decoded_buffer, &mut stdout_lock)?;
        }

        Ok(())
    }
}
