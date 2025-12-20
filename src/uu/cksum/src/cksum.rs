// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo, bitlen

use std::ffi::OsStr;

use clap::Command;
use uucore::checksum::cli::{ChecksumCommand, checksum_main, default_checksum_app};
use uucore::checksum::compute::OutputFormat;
use uucore::checksum::{
    AlgoKind, ChecksumError, calculate_blake2b_length_str, cli::options,
    sanitize_sha2_sha3_length_str,
};
use uucore::error::UResult;
use uucore::hardware::{HasHardwareFeatures as _, SimdPolicy};
use uucore::{show_error, translate};

/// Print CPU hardware capability detection information to stderr
/// This matches GNU cksum's --debug behavior
fn print_cpu_debug_info() {
    let features = SimdPolicy::detect();

    fn print_feature(name: &str, available: bool) {
        if available {
            show_error!("using {name} hardware support");
        } else {
            show_error!("{name} support not detected");
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

        // For BLAKE2b, if a length is provided, validate it.
        (Some(AlgoKind::Blake2b), Some(len)) => calculate_blake2b_length_str(len),

        // For any other provided algorithm, check if length is 0.
        // Otherwise, this is an error.
        (_, Some(len)) if len.parse::<u32>() == Ok(0_u32) => Ok(None),
        (_, Some(_)) => Err(ChecksumError::LengthOnlyForBlake2bSha2Sha3.into()),
    }
}

/// cksum has a bunch of legacy behavior. We handle this in this function to
/// make sure they are self contained and "easier" to understand.
///
/// Returns a pair of boolean. The first one indicates if we should use tagged
/// output format, the second one indicates if we should use the binary flag in
/// the untagged case.
fn handle_tag_text_binary_flags<S: AsRef<OsStr>>(
    args: impl Iterator<Item = S>,
) -> UResult<(bool, bool)> {
    let mut tag = true;
    let mut binary = false;
    let mut text = false;

    // --binary, --tag and --untagged are tight together: none of them
    // conflicts with each other but --tag will reset "binary" and "text" and
    // set "tag".

    for arg in args {
        let arg = arg.as_ref();
        if arg == "-b" || arg == "--binary" {
            text = false;
            binary = true;
        } else if arg == "--text" {
            text = true;
            binary = false;
        } else if arg == "--tag" {
            tag = true;
            binary = false;
            text = false;
        } else if arg == "--untagged" {
            tag = false;
        }
    }

    // Specifying --text without ever mentioning --untagged fails.
    if text && tag {
        return Err(ChecksumError::TextWithoutUntagged.into());
    }

    Ok((tag, binary))
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

    let (tag, binary) = handle_tag_text_binary_flags(std::env::args_os())?;

    let output_format = OutputFormat::from_cksum(
        algo_cli.unwrap_or(AlgoKind::Crc),
        tag,
        binary,
        /* raw: */
        matches.get_flag(options::RAW),
        /* base64: */
        matches.get_flag(options::BASE64),
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
        .with_length()
        .with_check()
        .with_untagged()
        .with_tag(true)
        .with_raw()
        .with_base64()
        .with_text(false)
        .with_binary()
        .with_zero()
        .with_debug()
        .after_help(translate!("cksum-after-help"))
}
