// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo, bitlen

use std::io::{Write, stderr};

use clap::Command;
use uu_checksum_common::{ChecksumCommand, checksum_main, default_checksum_app, options};

use uucore::checksum::compute::OutputFormat;
use uucore::checksum::{
    AlgoKind, ChecksumError, calculate_blake2b_length_str, sanitize_sha2_sha3_length_str,
};
use uucore::error::UResult;
use uucore::hardware::{HasHardwareFeatures as _, SimdPolicy};
use uucore::translate;

/// Print CPU hardware capability detection information to stderr
/// 2>/dev/full does not abort
/// This matches GNU cksum's --debug behavior
fn print_cpu_debug_info() {
    let features = SimdPolicy::detect();

    fn print_feature(name: &str, available: bool) {
        if available {
            let _ = writeln!(stderr(), "using {name} hardware support");
        } else {
            let _ = writeln!(stderr(), "{name} support not detected");
        }
    }

    // x86/x86_64
    print_feature("avx512", features.has_avx512());
    print_feature("avx2", features.has_avx2());
    print_feature("pclmul", features.has_pclmul());

    // ARM aarch64
    if cfg!(target_arch = "aarch64") {
        print_feature("vmull", features.has_vmull());
    }
}

/// Sanitize the `--length` argument depending on `--algorithm` and `--length`.
fn maybe_sanitize_length(
    algo_cli: Option<AlgoKind>,
    input_length: Option<&str>,
) -> UResult<Option<usize>> {
    match (algo_cli, input_length) {
        // No provided length is not a problem so far.
        (_, None) => Ok(None),

        // For SHA2 and SHA3, if a length is provided, ensure it is correct.
        (Some(algo @ (AlgoKind::Sha2 | AlgoKind::Sha3)), Some(s_len)) => {
            sanitize_sha2_sha3_length_str(algo, s_len).map(Some)
        }

        // SHAKE128 and SHAKE256 algorithms optionally take a bit length. No
        // validation is performed on this length, any value is valid. If the
        // given length is not a multiple of 8, the last byte of the output
        // will have its extra bits set to zero.
        (Some(AlgoKind::Shake128 | AlgoKind::Shake256), Some(len)) => match len.parse::<usize>() {
            Ok(0) => Ok(None),
            Ok(l) => Ok(Some(l)),
            Err(_) => Err(ChecksumError::InvalidLength(len.into()).into()),
        },

        // For BLAKE2b, if a length is provided, validate it.
        (Some(AlgoKind::Blake2b), Some(len)) => calculate_blake2b_length_str(len),

        // For any other provided algorithm, check if length is 0.
        // Otherwise, this is an error.
        (_, Some(len)) if len.parse::<u32>() == Ok(0_u32) => Ok(None),
        (_, Some(_)) => Err(ChecksumError::LengthOnlyForBlake2bSha2Sha3.into()),
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let algo_cli = matches
        .get_one::<String>(options::ALGORITHM)
        .map(AlgoKind::from_cksum)
        .transpose()?;

    let input_length = matches
        .get_one::<String>(options::LENGTH)
        .map(String::as_str);

    let length = maybe_sanitize_length(algo_cli, input_length)?;
    let tag = !matches.get_flag(options::UNTAGGED);
    let binary = matches.get_flag(options::BINARY);
    let text = matches.get_flag(options::TEXT);

    //Specifying --text without ever mentioning --untagged fails.
    if text && tag {
        return Err(ChecksumError::TextWithoutUntagged.into());
    }

    let output_format = OutputFormat::from_cksum(
        algo_cli.unwrap_or(AlgoKind::Crc),
        tag,
        binary,
        /* raw */ matches.get_flag(options::RAW),
        /* base64 */ matches.get_flag(options::BASE64),
    );

    // Print hardware debug info if requested
    if matches.get_flag(options::DEBUG) {
        print_cpu_debug_info();
    }

    checksum_main(algo_cli, length, matches, output_format)
}

pub fn uu_app() -> Command {
    default_checksum_app(translate!("cksum-about"), translate!("cksum-usage"))
        .with_algo()
        .with_untagged()
        .with_tag(true)
        .with_length()
        .with_raw()
        .with_check_and_opts()
        .with_base64()
        .with_text(false)
        .with_binary()
        .with_zero()
        .with_debug()
        .after_help(translate!("cksum-after-help"))
}
