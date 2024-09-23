// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore HEXUPPER Lsbf Msbf

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{Read, Stdin};
use std::path::Path;
use uucore::display::Quotable;
use uucore::encoding::{
    for_fast_encode::{BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER},
    Format, ZEightFiveWrapper, BASE2LSBF, BASE2MSBF,
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
            // Do not buffer input, because buffering is handled by `fast_decode` and `fast_encode`
            let file_buf =
                File::open(Path::new(name)).map_err_context(|| name.maybe_quote().to_string())?;
            Ok(Box::new(file_buf))
        }
        None => Ok(Box::new(stdin_ref.lock())),
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
    //
    // The encoding performed by `fast_encode` depend on these constants being correct. Performance can be tuned by
    // multiplying these numbers by a different multiple (see `DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE` above).
    const BASE16_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE2_UN_PADDED_MULTIPLE: usize = 1_usize;
    const BASE32_UN_PADDED_MULTIPLE: usize = 5_usize;
    const BASE64_UN_PADDED_MULTIPLE: usize = 3_usize;
    const Z85_UN_PADDED_MULTIPLE: usize = 4_usize;

    // Similar to above, but for decoding
    const BASE16_VALID_DECODING_MULTIPLE: usize = 2_usize;
    const BASE2_VALID_DECODING_MULTIPLE: usize = 8_usize;
    const BASE32_VALID_DECODING_MULTIPLE: usize = 8_usize;
    const BASE64_VALID_DECODING_MULTIPLE: usize = 4_usize;
    const Z85_VALID_DECODING_MULTIPLE: usize = 5_usize;

    if decode {
        let (encoding, valid_decoding_multiple, alphabet): (_, _, &[u8]) = match format {
            // Use naive approach (now only semi-naive) for Z85, since the crate being used doesn't have the API
            // needed
            Format::Z85 => {
                // spell-checker:disable-next-line
                let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

                fast_decode::fast_decode(
                    input,
                    (
                        ZEightFiveWrapper {},
                        Z85_VALID_DECODING_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE,
                        alphabet,
                    ),
                    ignore_garbage,
                )?;

                return Ok(());
            }

            // For these, use faster, new decoding logic
            Format::Base16 => (
                HEXUPPER,
                BASE16_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"0123456789ABCDEF",
            ),
            Format::Base2Lsbf => (
                BASE2LSBF,
                BASE2_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"01",
            ),
            Format::Base2Msbf => (
                BASE2MSBF,
                BASE2_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"01",
            ),
            Format::Base32 => (
                BASE32,
                BASE32_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
            ),
            Format::Base32Hex => (
                BASE32HEX,
                BASE32_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"0123456789ABCDEFGHIJKLMNOPQRSTUV=",
            ),
            Format::Base64 => (
                BASE64,
                BASE64_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=+/",
            ),
            Format::Base64Url => (
                BASE64URL,
                BASE64_VALID_DECODING_MULTIPLE,
                // spell-checker:disable-next-line
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=_-",
            ),
        };

        fast_decode::fast_decode(
            input,
            (
                encoding,
                valid_decoding_multiple * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE,
                alphabet,
            ),
            ignore_garbage,
        )?;

        Ok(())
    } else {
        let (encoding, un_padded_multiple) = match format {
            // Use naive approach for Z85 (now only semi-naive), since the crate being used doesn't have the API
            // needed
            Format::Z85 => {
                fast_encode::fast_encode(
                    input,
                    (
                        ZEightFiveWrapper {},
                        Z85_UN_PADDED_MULTIPLE * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE,
                    ),
                    wrap,
                )?;

                return Ok(());
            }

            // For these, use faster, new encoding logic
            Format::Base16 => (HEXUPPER, BASE16_UN_PADDED_MULTIPLE),
            Format::Base2Lsbf => (BASE2LSBF, BASE2_UN_PADDED_MULTIPLE),
            Format::Base2Msbf => (BASE2MSBF, BASE2_UN_PADDED_MULTIPLE),
            Format::Base32 => (BASE32, BASE32_UN_PADDED_MULTIPLE),
            Format::Base32Hex => (BASE32HEX, BASE32_UN_PADDED_MULTIPLE),
            Format::Base64 => (BASE64, BASE64_UN_PADDED_MULTIPLE),
            Format::Base64Url => (BASE64URL, BASE64_UN_PADDED_MULTIPLE),
        };

        fast_encode::fast_encode(
            input,
            (
                encoding,
                un_padded_multiple * DECODE_AND_ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE,
            ),
            wrap,
        )?;

        Ok(())
    }
}

mod fast_encode {
    use crate::base_common::WRAP_DEFAULT;
    use std::{
        collections::VecDeque,
        io::{self, ErrorKind, Read, StdoutLock, Write},
        num::NonZeroUsize,
    };
    use uucore::{
        encoding::SupportsFastEncode,
        error::{UResult, USimpleError},
    };

    struct LineWrapping {
        line_length: NonZeroUsize,
        print_buffer: Vec<u8>,
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
        let line_length_usize = line_length.get();

        let make_contiguous_result = encoded_buffer.make_contiguous();

        let chunks_exact = make_contiguous_result.chunks_exact(line_length_usize);

        let mut bytes_added_to_print_buffer = 0_usize;

        for sl in chunks_exact {
            bytes_added_to_print_buffer += sl.len();

            print_buffer.extend_from_slice(sl);
            print_buffer.push(b'\n');
        }

        stdout_lock.write_all(print_buffer)?;

        // Remove the bytes that were just printed from `encoded_buffer`
        drop(encoded_buffer.drain(..bytes_added_to_print_buffer));

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
    pub fn fast_encode<R: Read, S: SupportsFastEncode>(
        input: &mut R,
        (supports_fast_encode, encode_in_chunks_of_size): (S, usize),
        line_wrap: Option<usize>,
    ) -> UResult<()> {
        // Based on performance testing
        const INPUT_BUFFER_SIZE: usize = 32_usize * 1_024_usize;

        let mut line_wrapping_option = match line_wrap {
            // Line wrapping is disabled because "-w"/"--wrap" was passed with "0"
            Some(0_usize) => None,
            // A custom line wrapping value was passed
            Some(an) => Some(LineWrapping {
                line_length: NonZeroUsize::new(an).unwrap(),
                print_buffer: Vec::<u8>::new(),
            }),
            // Line wrapping was not set, so the default is used
            None => Some(LineWrapping {
                line_length: NonZeroUsize::new(WRAP_DEFAULT).unwrap(),
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
                            supports_fast_encode.encode_to_vec_deque(
                                leftover_buffer.make_contiguous(),
                                &mut encoded_buffer,
                            )?;

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

                            supports_fast_encode.encode_to_vec_deque(sl, &mut encoded_buffer)?;
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
            supports_fast_encode
                .encode_to_vec_deque(leftover_buffer.make_contiguous(), &mut encoded_buffer)?;

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
        encoding::SupportsFastDecode,
        error::{UResult, USimpleError},
    };

    // Start of helper functions
    pub fn alphabet_to_table(alphabet: &[u8], ignore_garbage: bool) -> [bool; 256_usize] {
        // If "ignore_garbage" is enabled, all characters outside the alphabet are ignored
        // If it is not enabled, only '\n' and '\r' are ignored
        if ignore_garbage {
            // Note: "false" here
            let mut table = [false; 256_usize];

            // Pass through no characters except those in the alphabet
            for ue in alphabet {
                let us = usize::from(*ue);

                // Should not have been set yet
                assert!(!table[us]);

                table[us] = true;
            }

            table
        } else {
            // Note: "true" here
            let mut table = [true; 256_usize];

            // Pass through all characters except '\n' and '\r'
            for ue in [b'\n', b'\r'] {
                let us = usize::from(ue);

                // Should not have been set yet
                assert!(table[us]);

                table[us] = false;
            }

            table
        }
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
    pub fn fast_decode<R: Read, S: SupportsFastDecode>(
        input: &mut R,
        (supports_fast_decode, decode_in_chunks_of_size, alphabet): (S, usize, &[u8]),
        ignore_garbage: bool,
    ) -> UResult<()> {
        // Based on performance testing
        const INPUT_BUFFER_SIZE: usize = 32_usize * 1_024_usize;

        // Note that it's not worth using "data-encoding"'s ignore functionality if "ignore_garbage" is true, because
        // "data-encoding"'s ignore functionality cannot discard non-ASCII bytes. The data has to be filtered before
        // passing it to "data-encoding", so there is no point in doing any filtering in "data-encoding". This also
        // allows execution to stay on the happy path in "data-encoding":
        // https://github.com/ia0/data-encoding/blob/4f42ad7ef242f6d243e4de90cd1b46a57690d00e/lib/src/lib.rs#L754-L756
        // Update: it is not even worth it to use "data-encoding"'s ignore functionality when "ignore_garbage" is
        // false.
        // Note that the alphabet constants above already include the padding characters
        // TODO
        // Precompute this
        let table = alphabet_to_table(alphabet, ignore_garbage);

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

                        // First just scan the data for the happy path
                        // Yields significant speedup when the input does not contain line endings
                        let found_garbage = read_buffer.iter().any(|ue| {
                            // Garbage, since it was not found in the table
                            !table[usize::from(*ue)]
                        });

                        if found_garbage {
                            non_garbage_buffer.clear();

                            for ue in read_buffer {
                                if table[usize::from(*ue)] {
                                    // Not garbage, since it was found in the table
                                    non_garbage_buffer.push(*ue);
                                }
                            }

                            non_garbage_buffer.as_slice()
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
                            supports_fast_decode
                                .decode_into_vec(&leftover_buffer, &mut decoded_buffer)?;

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

                            supports_fast_decode.decode_into_vec(sl, &mut decoded_buffer)?;
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
            supports_fast_decode.decode_into_vec(&leftover_buffer, &mut decoded_buffer)?;

            // Write all data in `decoded_buffer` to stdout
            write_to_stdout(&mut decoded_buffer, &mut stdout_lock)?;
        }

        Ok(())
    }
}
