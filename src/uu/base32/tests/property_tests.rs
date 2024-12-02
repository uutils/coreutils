// spell-checker:ignore lsbf msbf proptest

use proptest::{prelude::TestCaseError, prop_assert, prop_assert_eq, test_runner::TestRunner};
use std::io::Cursor;
use uu_base32::base_common::{fast_decode, fast_encode, get_supports_fast_decode_and_encode};
use uucore::encoding::{Format, SupportsFastDecodeAndEncode};

const CASES: u32 = {
    #[cfg(debug_assertions)]
    {
        32
    }

    #[cfg(not(debug_assertions))]
    {
        128
    }
};

const NORMAL_INPUT_SIZE_LIMIT: usize = {
    #[cfg(debug_assertions)]
    {
        // 256 kibibytes
        256 * 1024
    }

    #[cfg(not(debug_assertions))]
    {
        // 4 mebibytes
        4 * 1024 * 1024
    }
};

const LARGE_INPUT_SIZE_LIMIT: usize = 4 * NORMAL_INPUT_SIZE_LIMIT;

// Note that `TestRunner`s cannot be reused
fn get_test_runner() -> TestRunner {
    TestRunner::new(proptest::test_runner::Config {
        cases: CASES,
        failure_persistence: None,

        ..proptest::test_runner::Config::default()
    })
}

fn generic_round_trip(format: Format) {
    let supports_fast_decode_and_encode = get_supports_fast_decode_and_encode(format);

    let supports_fast_decode_and_encode_ref = supports_fast_decode_and_encode.as_ref();

    // Make sure empty inputs round trip
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        // Do not add garbage
                        Vec::<(usize, u8)>::new(),
                        // Empty input
                        Vec::<u8>::new(),
                    )
                },
            )
            .unwrap();
    }

    // Unusually large line wrapping settings
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(512_usize..65_535_usize),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..NORMAL_INPUT_SIZE_LIMIT),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        // Do not add garbage
                        Vec::<(usize, u8)>::new(),
                        input,
                    )
                },
            )
            .unwrap();
    }

    // Spend more time on sane line wrapping settings
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..NORMAL_INPUT_SIZE_LIMIT),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        // Do not add garbage
                        Vec::<(usize, u8)>::new(),
                        input,
                    )
                },
            )
            .unwrap();
    }

    // Test with garbage data
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                    // Garbage data to insert
                    proptest::collection::vec(
                        (
                            // Random index
                            proptest::num::usize::ANY,
                            // In all of the encodings being tested, non-ASCII bytes are garbage
                            128_u8..=u8::MAX,
                        ),
                        0..4_096,
                    ),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..NORMAL_INPUT_SIZE_LIMIT),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, garbage_data, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        garbage_data,
                        input,
                    )
                },
            )
            .unwrap();
    }

    // Test small inputs
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..1_024),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        // Do not add garbage
                        Vec::<(usize, u8)>::new(),
                        input,
                    )
                },
            )
            .unwrap();
    }

    // Test small inputs with garbage data
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                    // Garbage data to insert
                    proptest::collection::vec(
                        (
                            // Random index
                            proptest::num::usize::ANY,
                            // In all of the encodings being tested, non-ASCII bytes are garbage
                            128_u8..=u8::MAX,
                        ),
                        0..1_024,
                    ),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..1_024),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, garbage_data, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        garbage_data,
                        input,
                    )
                },
            )
            .unwrap();
    }

    // Test large inputs
    {
        get_test_runner()
            .run(
                &(
                    proptest::bool::ANY,
                    proptest::bool::ANY,
                    proptest::option::of(0_usize..512_usize),
                    proptest::collection::vec(proptest::num::u8::ANY, 0..LARGE_INPUT_SIZE_LIMIT),
                ),
                |(ignore_garbage, line_wrap_zero, line_wrap, input)| {
                    configurable_round_trip(
                        format,
                        supports_fast_decode_and_encode_ref,
                        ignore_garbage,
                        line_wrap_zero,
                        line_wrap,
                        // Do not add garbage
                        Vec::<(usize, u8)>::new(),
                        input,
                    )
                },
            )
            .unwrap();
    }
}

fn configurable_round_trip(
    format: Format,
    supports_fast_decode_and_encode: &dyn SupportsFastDecodeAndEncode,
    ignore_garbage: bool,
    line_wrap_zero: bool,
    line_wrap: Option<usize>,
    garbage_data: Vec<(usize, u8)>,
    mut input: Vec<u8>,
) -> Result<(), TestCaseError> {
    // Z85 only accepts inputs with lengths divisible by 4
    if let Format::Z85 = format {
        // Reduce length of "input" until it is divisible by 4
        input.truncate((input.len() / 4) * 4);

        assert!((input.len() % 4) == 0);
    }

    let line_wrap_to_use = if line_wrap_zero { Some(0) } else { line_wrap };

    let input_len = input.len();

    let garbage_data_len = garbage_data.len();

    let garbage_data_is_empty = garbage_data_len == 0;

    let (input, encoded) = {
        let mut output = Vec::with_capacity(input_len * 8);

        let mut cursor = Cursor::new(input);

        fast_encode::fast_encode(
            &mut cursor,
            &mut output,
            supports_fast_decode_and_encode,
            line_wrap_to_use,
        )
        .unwrap();

        (cursor.into_inner(), output)
    };

    let encoded_or_encoded_with_garbage = if garbage_data_is_empty {
        encoded
    } else {
        let encoded_len = encoded.len();

        let encoded_highest_index = match encoded_len.checked_sub(1) {
            Some(0) | None => None,
            Some(x) => Some(x),
        };

        let mut garbage_data_indexed = vec![Option::<u8>::None; encoded_len];

        let mut encoded_with_garbage = Vec::<u8>::with_capacity(encoded_len + garbage_data_len);

        for (index, garbage_byte) in garbage_data {
            if let Some(x) = encoded_highest_index {
                let index_to_use = index % x;

                garbage_data_indexed[index_to_use] = Some(garbage_byte);
            } else {
                encoded_with_garbage.push(garbage_byte);
            }
        }

        for (index, encoded_byte) in encoded.into_iter().enumerate() {
            encoded_with_garbage.push(encoded_byte);

            if let Some(garbage_byte) = garbage_data_indexed[index] {
                encoded_with_garbage.push(garbage_byte);
            }
        }

        encoded_with_garbage
    };

    match line_wrap_to_use {
        Some(0) => {
            let line_endings_count = encoded_or_encoded_with_garbage
                .iter()
                .filter(|byte| **byte == b'\n')
                .count();

            // If line wrapping is disabled, there should only be one '\n' character (at the very end of the output)
            prop_assert_eq!(line_endings_count, 1);
        }
        _ => {
            // TODO
            // Validate other line wrapping settings
        }
    }

    let decoded_or_error = {
        let mut output = Vec::with_capacity(input_len);

        let mut cursor = Cursor::new(encoded_or_encoded_with_garbage);

        match fast_decode::fast_decode(
            &mut cursor,
            &mut output,
            supports_fast_decode_and_encode,
            ignore_garbage,
        ) {
            Ok(()) => Ok(output),
            Err(er) => Err(er),
        }
    };

    let made_round_trip = match decoded_or_error {
        Ok(ve) => input.as_slice() == ve.as_slice(),
        Err(_) => false,
    };

    let result_was_correct = if garbage_data_is_empty || ignore_garbage {
        // If there was no garbage data added, or if "ignore_garbage" was enabled, expect the round trip to succeed
        made_round_trip
    } else {
        // If garbage data was added, and "ignore_garbage" was disabled, expect the round trip to fail

        !made_round_trip
    };

    if !result_was_correct {
        eprintln!(
            "\
(configurable_round_trip) FAILURE
format: {format:?}
ignore_garbage: {ignore_garbage}
line_wrap_to_use: {line_wrap_to_use:?}
garbage_data_len: {garbage_data_len}
input_len: {input_len}
",
        );
    }

    prop_assert!(result_was_correct);

    Ok(())
}

#[test]
fn base16_round_trip() {
    generic_round_trip(Format::Base16);
}

#[test]
fn base2lsbf_round_trip() {
    generic_round_trip(Format::Base2Lsbf);
}

#[test]
fn base2msbf_round_trip() {
    generic_round_trip(Format::Base2Msbf);
}

#[test]
fn base32_round_trip() {
    generic_round_trip(Format::Base32);
}

#[test]
fn base32hex_round_trip() {
    generic_round_trip(Format::Base32Hex);
}

#[test]
fn base64_round_trip() {
    generic_round_trip(Format::Base64);
}

#[test]
fn base64url_round_trip() {
    generic_round_trip(Format::Base64Url);
}

#[test]
fn z85_round_trip() {
    generic_round_trip(Format::Z85);
}
