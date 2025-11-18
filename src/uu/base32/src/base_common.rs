// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore hexupper lsbf msbf unpadded nopad aGVsbG8sIHdvcmxkIQ

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::encoding::{
    BASE2LSBF, BASE2MSBF, Base32Wrapper, Base58Wrapper, Base64SimdWrapper, EncodingWrapper, Format,
    SupportsFastDecodeAndEncode, Z85Wrapper,
    for_base_common::{BASE32, BASE32HEX, BASE64URL, HEXUPPER_PERMISSIVE},
};
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::translate;

pub const BASE_CMD_PARSE_ERROR: i32 = 1;

/// Encoded output will be formatted in lines of this length (the last line can be shorter)
///
/// Other implementations default to 76
///
/// This default is only used if no "-w"/"--wrap" argument is passed
pub const WRAP_DEFAULT: usize = 76;

pub struct Config {
    pub decode: bool,
    pub ignore_garbage: bool,
    pub wrap_cols: Option<usize>,
    pub to_read: Option<PathBuf>,
}

pub mod options {
    pub static DECODE: &str = "decode";
    pub static WRAP: &str = "wrap";
    pub static IGNORE_GARBAGE: &str = "ignore-garbage";
    pub static FILE: &str = "file";
}

impl Config {
    pub fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let to_read = match options.get_many::<OsString>(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();

                if let Some(extra_op) = values.next() {
                    return Err(UUsageError::new(
                        BASE_CMD_PARSE_ERROR,
                        translate!("base-common-extra-operand", "operand" => extra_op.to_string_lossy().quote()),
                    ));
                }

                if name == "-" {
                    None
                } else {
                    let path = Path::new(name);

                    if !path.exists() {
                        return Err(USimpleError::new(
                            BASE_CMD_PARSE_ERROR,
                            translate!("base-common-no-such-file", "file" => path.maybe_quote()),
                        ));
                    }

                    Some(path.to_owned())
                }
            }
            None => None,
        };

        let wrap_cols = options
            .get_one::<String>(options::WRAP)
            .map(|num| {
                num.parse::<usize>().map_err(|_| {
                    USimpleError::new(
                        BASE_CMD_PARSE_ERROR,
                        translate!("base-common-invalid-wrap-size", "size" => num.quote()),
                    )
                })
            })
            .transpose()?;

        Ok(Self {
            decode: options.get_flag(options::DECODE),
            ignore_garbage: options.get_flag(options::IGNORE_GARBAGE),
            wrap_cols,
            to_read,
        })
    }
}

pub fn parse_base_cmd_args(
    args: impl uucore::Args,
    about: &'static str,
    usage: &str,
) -> UResult<Config> {
    let command = base_app(about, usage);
    let matches = uucore::clap_localization::handle_clap_result(command, args)?;
    Config::from(&matches)
}

pub fn base_app(about: &'static str, usage: &str) -> Command {
    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(about)
        .override_usage(format_usage(usage))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        // Format arguments.
        .arg(
            Arg::new(options::DECODE)
                .short('d')
                .visible_short_alias('D')
                .long(options::DECODE)
                .help(translate!("base-common-help-decode"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::DECODE),
        )
        .arg(
            Arg::new(options::IGNORE_GARBAGE)
                .short('i')
                .long(options::IGNORE_GARBAGE)
                .help(translate!("base-common-help-ignore-garbage"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::IGNORE_GARBAGE),
        )
        .arg(
            Arg::new(options::WRAP)
                .short('w')
                .long(options::WRAP)
                .value_name("COLS")
                .help(translate!("base-common-help-wrap", "default" => WRAP_DEFAULT))
                .overrides_with(options::WRAP),
        )
        // "multiple" arguments are used to check whether there is more than one
        // file passed in.
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
}

pub fn get_input(config: &Config) -> UResult<Box<dyn Read>> {
    match &config.to_read {
        Some(path_buf) => {
            // Do not buffer input, because buffering is handled by `fast_decode` and `fast_encode`
            let file =
                File::open(path_buf).map_err_context(|| path_buf.maybe_quote().to_string())?;
            Ok(Box::new(file))
        }
        None => Ok(Box::new(io::stdin())),
    }
}

pub fn handle_input<R: Read>(input: &mut R, format: Format, config: Config) -> UResult<()> {
    let supports_fast_decode_and_encode = get_supports_fast_decode_and_encode(format);

    let supports_fast_decode_and_encode_ref = supports_fast_decode_and_encode.as_ref();
    let mut stdout_lock = io::stdout().lock();
    let result = if config.decode {
        fast_decode::fast_decode(
            input,
            &mut stdout_lock,
            supports_fast_decode_and_encode_ref,
            config.ignore_garbage,
        )
    } else {
        fast_encode::fast_encode(
            input,
            &mut stdout_lock,
            supports_fast_decode_and_encode_ref,
            config.wrap_cols,
        )
    };

    // Ensure any pending stdout buffer is flushed even if decoding failed; GNU basenc
    // keeps already-decoded bytes visible before reporting the error.
    match (result, stdout_lock.flush()) {
        (res, Ok(())) => res,
        (Ok(_), Err(err)) => Err(err.into()),
        (Err(original), Err(_)) => Err(original),
    }
}

pub fn get_supports_fast_decode_and_encode(format: Format) -> Box<dyn SupportsFastDecodeAndEncode> {
    const BASE16_VALID_DECODING_MULTIPLE: usize = 2;
    const BASE2_VALID_DECODING_MULTIPLE: usize = 8;
    const BASE32_VALID_DECODING_MULTIPLE: usize = 8;
    const BASE64_VALID_DECODING_MULTIPLE: usize = 4;

    const BASE16_UNPADDED_MULTIPLE: usize = 1;
    const BASE2_UNPADDED_MULTIPLE: usize = 1;
    const BASE32_UNPADDED_MULTIPLE: usize = 5;
    const BASE64_UNPADDED_MULTIPLE: usize = 3;

    match format {
        Format::Base16 => Box::from(EncodingWrapper::new(
            HEXUPPER_PERMISSIVE,
            BASE16_VALID_DECODING_MULTIPLE,
            BASE16_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"0123456789ABCDEFabcdef",
        )),
        Format::Base2Lsbf => Box::from(EncodingWrapper::new(
            BASE2LSBF,
            BASE2_VALID_DECODING_MULTIPLE,
            BASE2_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"01",
        )),
        Format::Base2Msbf => Box::from(EncodingWrapper::new(
            BASE2MSBF,
            BASE2_VALID_DECODING_MULTIPLE,
            BASE2_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"01",
        )),
        Format::Base32 => Box::from(Base32Wrapper::new(
            BASE32,
            BASE32_VALID_DECODING_MULTIPLE,
            BASE32_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
        )),
        Format::Base32Hex => Box::from(Base32Wrapper::new(
            BASE32HEX,
            BASE32_VALID_DECODING_MULTIPLE,
            BASE32_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"0123456789ABCDEFGHIJKLMNOPQRSTUV=",
        )),
        Format::Base64 => Box::from(Base64SimdWrapper::new(
            BASE64_VALID_DECODING_MULTIPLE,
            BASE64_UNPADDED_MULTIPLE,
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=",
        )),
        Format::Base64Url => Box::from(EncodingWrapper::new(
            BASE64URL,
            BASE64_VALID_DECODING_MULTIPLE,
            BASE64_UNPADDED_MULTIPLE,
            // spell-checker:disable-next-line
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=_-",
        )),
        Format::Z85 => Box::from(Z85Wrapper {}),
        Format::Base58 => Box::from(Base58Wrapper {}),
    }
}

pub mod fast_encode {
    use crate::base_common::{WRAP_DEFAULT, format_read_error};
    use std::{
        collections::VecDeque,
        io::{self, BufRead, BufReader, ErrorKind, Read, Write},
        num::NonZeroUsize,
    };
    use uucore::{
        encoding::SupportsFastDecodeAndEncode,
        error::{UResult, USimpleError},
    };

    struct LineWrapping {
        line_length: NonZeroUsize,
        print_buffer: Vec<u8>,
    }

    // Start of helper functions
    fn encode_in_chunks_to_buffer(
        supports_fast_decode_and_encode: &dyn SupportsFastDecodeAndEncode,
        read_buffer: &[u8],
        encoded_buffer: &mut VecDeque<u8>,
    ) -> UResult<()> {
        supports_fast_decode_and_encode.encode_to_vec_deque(read_buffer, encoded_buffer)?;
        Ok(())
    }

    fn write_without_line_breaks(
        encoded_buffer: &mut VecDeque<u8>,
        output: &mut dyn Write,
        is_cleanup: bool,
        empty_wrap: bool,
    ) -> io::Result<()> {
        // TODO
        // `encoded_buffer` only has to be a VecDeque if line wrapping is enabled
        // (`make_contiguous` should be a no-op here)
        // Refactoring could avoid this call
        output.write_all(encoded_buffer.make_contiguous())?;

        if is_cleanup {
            if !empty_wrap {
                output.write_all(b"\n")?;
            }
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
        output: &mut dyn Write,
        is_cleanup: bool,
    ) -> io::Result<()> {
        let line_length = line_length.get();

        let make_contiguous_result = encoded_buffer.make_contiguous();

        let chunks_exact = make_contiguous_result.chunks_exact(line_length);

        let mut bytes_added_to_print_buffer = 0;

        for sl in chunks_exact {
            bytes_added_to_print_buffer += sl.len();

            print_buffer.extend_from_slice(sl);
            print_buffer.push(b'\n');
        }

        output.write_all(print_buffer)?;

        // Remove the bytes that were just printed from `encoded_buffer`
        drop(encoded_buffer.drain(..bytes_added_to_print_buffer));

        if is_cleanup {
            if encoded_buffer.is_empty() {
                // Do not write a newline in this case, because two trailing newlines should never be printed
            } else {
                // Print the partial line, since this is cleanup and no more data is coming
                output.write_all(encoded_buffer.make_contiguous())?;
                output.write_all(b"\n")?;
            }
        } else {
            print_buffer.clear();
        }

        Ok(())
    }

    fn write_to_output(
        line_wrapping: &mut Option<LineWrapping>,
        encoded_buffer: &mut VecDeque<u8>,
        output: &mut dyn Write,
        is_cleanup: bool,
        empty_wrap: bool,
    ) -> io::Result<()> {
        // Write all data in `encoded_buffer` to `output`
        if let &mut Some(ref mut li) = line_wrapping {
            write_with_line_breaks(li, encoded_buffer, output, is_cleanup)?;
        } else {
            write_without_line_breaks(encoded_buffer, output, is_cleanup, empty_wrap)?;
        }

        Ok(())
    }
    // End of helper functions

    pub fn fast_encode(
        input: &mut dyn Read,
        output: &mut dyn Write,
        supports_fast_decode_and_encode: &dyn SupportsFastDecodeAndEncode,
        wrap: Option<usize>,
    ) -> UResult<()> {
        // Based on performance testing

        const ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE: usize = 1_024;
        let mut encode_in_chunks_of_size = 0;
        if supports_fast_decode_and_encode.should_buffer_encoding() {
            encode_in_chunks_of_size = supports_fast_decode_and_encode.unpadded_multiple()
                * ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
        }

        assert!(
            (encode_in_chunks_of_size > 0
                && supports_fast_decode_and_encode.should_buffer_decoding())
                || !supports_fast_decode_and_encode.should_buffer_encoding()
        );

        // The "data-encoding" crate supports line wrapping, but not arbitrary line wrapping, only certain widths, so
        // line wrapping must be handled here.
        // https://github.com/ia0/data-encoding/blob/4f42ad7ef242f6d243e4de90cd1b46a57690d00e/lib/src/lib.rs#L1710
        let mut line_wrapping = match wrap {
            // Line wrapping is disabled because "-w"/"--wrap" was passed with "0"
            Some(0) => None,
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
        // Data that was read from `input` but has not been encoded yet
        let mut leftover_buffer = Vec::<u8>::new();
        // Encoded data that needs to be written to `output`
        let mut encoded_buffer = VecDeque::<u8>::new();
        // End of buffers
        if supports_fast_decode_and_encode.should_buffer_encoding() {
            let mut in_reader = BufReader::with_capacity(encode_in_chunks_of_size, input);
            loop {
                let buf_res = in_reader.fill_buf();
                if let Ok(buffer) = buf_res {
                    let buffer_len = buffer.len();
                    if buffer.len() < encode_in_chunks_of_size {
                        leftover_buffer.extend(buffer);
                        assert!(leftover_buffer.len() < encode_in_chunks_of_size);
                        break;
                    }
                    assert_eq!(buffer.len(), encode_in_chunks_of_size);
                    encode_in_chunks_to_buffer(
                        supports_fast_decode_and_encode,
                        buffer,
                        &mut encoded_buffer,
                    )?;
                    // Write all data in `encoded_buffer` to `output`
                    write_to_output(
                        &mut line_wrapping,
                        &mut encoded_buffer,
                        output,
                        false,
                        wrap == Some(0),
                    )?;
                    in_reader.consume(buffer_len);
                } else if let Err(err) = buf_res {
                    let kind = err.kind();

                    if kind == ErrorKind::Interrupted {
                        // Retry reading
                        continue;
                    }

                    return Err(USimpleError::new(1, format_read_error(kind)));
                }
            }
        } else {
            input.read_to_end(&mut leftover_buffer).unwrap();
        }
        // Cleanup
        // `input` has finished producing data, so the data remaining in the buffers needs to be encoded and printed
        // Encode all remaining unencoded bytes, placing them in `encoded_buffer`
        supports_fast_decode_and_encode
            .encode_to_vec_deque(&leftover_buffer, &mut encoded_buffer)?;
        // Write all data in `encoded_buffer` to output
        // `is_cleanup` triggers special cleanup-only logic
        write_to_output(
            &mut line_wrapping,
            &mut encoded_buffer,
            output,
            true,
            wrap == Some(0),
        )?;
        Ok(())
    }
}

pub mod fast_decode {
    use crate::base_common::format_read_error;
    use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
    use uucore::{
        encoding::SupportsFastDecodeAndEncode,
        error::{UResult, USimpleError},
    };

    // Start of helper functions
    fn alphabet_to_table(alphabet: &[u8], ignore_garbage: bool) -> [bool; 256] {
        // If `ignore_garbage` is enabled, all characters outside the alphabet are ignored
        // If it is not enabled, only '\n' and '\r' are ignored
        if ignore_garbage {
            // Note: "false" here
            let mut table = [false; 256];

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
            let mut table = [true; 256];

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

    fn decode_in_chunks_to_buffer(
        supports_fast_decode_and_encode: &dyn SupportsFastDecodeAndEncode,
        read_buffer_filtered: &[u8],
        decoded_buffer: &mut Vec<u8>,
    ) -> UResult<()> {
        supports_fast_decode_and_encode.decode_into_vec(read_buffer_filtered, decoded_buffer)?;
        Ok(())
    }

    fn write_to_output(decoded_buffer: &mut Vec<u8>, output: &mut dyn Write) -> io::Result<()> {
        // Write all data in `decoded_buffer` to `output`
        output.write_all(decoded_buffer.as_slice())?;
        output.flush()?;

        decoded_buffer.clear();

        Ok(())
    }

    pub fn fast_decode(
        input: &mut dyn Read,
        output: &mut dyn Write,
        supports_fast_decode_and_encode: &dyn SupportsFastDecodeAndEncode,
        ignore_garbage: bool,
    ) -> UResult<()> {
        const DECODE_IN_CHUNKS_OF_SIZE_MULTIPLE: usize = 1_024;
        let alphabet = supports_fast_decode_and_encode.alphabet();
        let mut decode_in_chunks_of_size = 0;
        if supports_fast_decode_and_encode.should_buffer_decoding() {
            decode_in_chunks_of_size = supports_fast_decode_and_encode.valid_decoding_multiple()
                * DECODE_IN_CHUNKS_OF_SIZE_MULTIPLE;
        }

        assert!(
            (decode_in_chunks_of_size > 0
                && supports_fast_decode_and_encode.should_buffer_decoding())
                || !supports_fast_decode_and_encode.should_buffer_decoding()
        );

        // Note that it's not worth using "data-encoding"'s ignore functionality if `ignore_garbage` is true, because
        // "data-encoding"'s ignore functionality cannot discard non-ASCII bytes. The data has to be filtered before
        // passing it to "data-encoding", so there is no point in doing any filtering in "data-encoding". This also
        // allows execution to stay on the happy path in "data-encoding":
        // https://github.com/ia0/data-encoding/blob/4f42ad7ef242f6d243e4de90cd1b46a57690d00e/lib/src/lib.rs#L754-L756
        // It is also not worth using "data-encoding"'s ignore functionality when `ignore_garbage` is
        // false.
        // Note that the alphabet constants above already include the padding characters
        // TODO
        // Precompute this
        let table = alphabet_to_table(alphabet, ignore_garbage);

        // Start of buffers

        // Decoded data that needs to be written to `output`
        let mut decoded_buffer = Vec::<u8>::new();
        let mut buffers = [
            Vec::with_capacity(decode_in_chunks_of_size),
            Vec::with_capacity(decode_in_chunks_of_size),
        ];
        let mut current_buffer_index = 0usize;
        if supports_fast_decode_and_encode.should_buffer_decoding() {
            let mut in_reader = BufReader::with_capacity(decode_in_chunks_of_size, input);
            // End of buffers
            loop {
                while buffers[current_buffer_index].len() < decode_in_chunks_of_size {
                    let read_res = in_reader.fill_buf();
                    if let Ok(read_buffer) = read_res {
                        let read_size = read_buffer.len();
                        if read_size == 0 {
                            break;
                        }
                        // Filter and fill the valid buffer. When it is filled, we
                        // switch buffer to avoid reading again.
                        read_buffer
                            .iter()
                            .filter(|ch| table[usize::from(**ch)])
                            .for_each(|ch| {
                                if buffers[current_buffer_index].len() < decode_in_chunks_of_size {
                                    buffers[current_buffer_index].push(*ch);
                                } else {
                                    buffers[(current_buffer_index + 1) % buffers.len()].push(*ch);
                                }
                            });
                        in_reader.consume(read_size);
                    } else if let Err(err) = read_res {
                        let kind = err.kind();

                        if kind == ErrorKind::Interrupted {
                            // Retry reading
                            continue;
                        }

                        return Err(USimpleError::new(1, format_read_error(kind)));
                    }
                }
                if buffers[current_buffer_index].len() < decode_in_chunks_of_size {
                    break;
                }
                assert_eq!(
                    buffers[current_buffer_index].len(),
                    decode_in_chunks_of_size
                );
                // Decode data in chunks, then place it in `decoded_buffer`
                decode_in_chunks_to_buffer(
                    supports_fast_decode_and_encode,
                    &buffers[current_buffer_index],
                    &mut decoded_buffer,
                )?;
                // Write all data in `decoded_buffer` to `output`
                write_to_output(&mut decoded_buffer, output)?;

                buffers[current_buffer_index].clear();
                current_buffer_index = (current_buffer_index + 1) % buffers.len();
            }
        } else {
            input
                .read_to_end(&mut buffers[current_buffer_index])
                .unwrap();
        }
        // Cleanup
        // `input` has finished producing data, so the data remaining in the buffers needs to be decoded and printed
        // Decode all remaining encoded bytes, placing them in `decoded_buffer`
        supports_fast_decode_and_encode
            .decode_into_vec(&buffers[current_buffer_index], &mut decoded_buffer)?;
        // Write all data in `decoded_buffer` to `output`
        write_to_output(&mut decoded_buffer, output)?;
        Ok(())
    }
}

fn format_read_error(kind: ErrorKind) -> String {
    let kind_string = kind.to_string();

    // e.g. "is a directory" -> "Is a directory"
    let mut kind_string_capitalized = String::with_capacity(kind_string.len());

    for (index, ch) in kind_string.char_indices() {
        if index == 0 {
            for cha in ch.to_uppercase() {
                kind_string_capitalized.push(cha);
            }
        } else {
            kind_string_capitalized.push(ch);
        }
    }

    translate!("base-common-read-error", "error" => kind_string_capitalized)
}

#[cfg(test)]
mod tests {
    use crate::base_common::read_and_has_padding;
    use std::io::Cursor;

    #[test]
    fn test_has_padding() {
        let test_cases = vec![
            ("aGVsbG8sIHdvcmxkIQ==", true),
            ("aGVsbG8sIHdvcmxkIQ== ", true),
            ("aGVsbG8sIHdvcmxkIQ==\n", true),
            ("aGVsbG8sIHdvcmxkIQ== \n", true),
            ("aGVsbG8sIHdvcmxkIQ=", true),
            ("aGVsbG8sIHdvcmxkIQ= ", true),
            ("MTIzNA==MTIzNA", true),
            ("MTIzNA==\nMTIzNA", true),
            ("aGVsbG8sIHdvcmxkIQ \n", false),
            ("aGVsbG8sIHdvcmxkIQ", false),
        ];

        for (input, expected) in test_cases {
            let mut cursor = Cursor::new(input.as_bytes());
            assert_eq!(
                read_and_has_padding(&mut cursor).unwrap().0,
                expected,
                "Failed for input: '{input}'"
            );
        }
    }
}
