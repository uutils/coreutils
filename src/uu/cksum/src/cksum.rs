// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo, bitlen

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::iter;
use uucore::checksum::compute::{
    ChecksumComputeOptions, figure_out_output_format, perform_checksum_computation,
};
use uucore::checksum::validate::{
    ChecksumValidateOptions, ChecksumVerbose, perform_checksum_validation,
};
use uucore::checksum::{
    AlgoKind, ChecksumError, SUPPORTED_ALGORITHMS, SizedAlgoKind, calculate_blake2b_length_str,
    sanitize_sha2_sha3_length_str,
};
use uucore::error::UResult;
use uucore::hardware::{HasHardwareFeatures as _, SimdPolicy};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, show_error, translate};

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

mod options {
    pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    pub const RAW: &str = "raw";
    pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const STRICT: &str = "strict";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const IGNORE_MISSING: &str = "ignore-missing";
    pub const QUIET: &str = "quiet";
    pub const ZERO: &str = "zero";
    pub const DEBUG: &str = "debug";
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let check = matches.get_flag(options::CHECK);

    let check_flag = |flag| match (check, matches.get_flag(flag)) {
        (_, false) => Ok(false),
        (true, true) => Ok(true),
        (false, true) => Err(ChecksumError::CheckOnlyFlag(flag.into())),
    };

    // Each of the following flags are only expected in --check mode.
    // If we encounter them otherwise, end with an error.
    let ignore_missing = check_flag(options::IGNORE_MISSING)?;
    let warn = check_flag(options::WARN)?;
    let quiet = check_flag(options::QUIET)?;
    let strict = check_flag(options::STRICT)?;
    let status = check_flag(options::STATUS)?;

    let algo_cli = matches
        .get_one::<String>(options::ALGORITHM)
        .map(AlgoKind::from_cksum)
        .transpose()?;

    let input_length = matches
        .get_one::<String>(options::LENGTH)
        .map(String::as_str);

    let length = maybe_sanitize_length(algo_cli, input_length)?;

    let files = matches.get_many::<OsString>(options::FILE).map_or_else(
        // No files given, read from stdin.
        || Box::new(iter::once(OsStr::new("-"))) as Box<dyn Iterator<Item = &OsStr>>,
        // At least one file given, read from them.
        |files| Box::new(files.map(OsStr::new)) as Box<dyn Iterator<Item = &OsStr>>,
    );

    if check {
        // cksum does not support '--check'ing legacy algorithms
        if algo_cli.is_some_and(AlgoKind::is_legacy) {
            return Err(ChecksumError::AlgorithmNotSupportedWithCheck.into());
        }

        let text_flag = matches.get_flag(options::TEXT);
        let binary_flag = matches.get_flag(options::BINARY);
        let tag = matches.get_flag(options::TAG);

        if tag || binary_flag || text_flag {
            return Err(ChecksumError::BinaryTextConflict.into());
        }

        // Execute the checksum validation based on the presence of files or the use of stdin

        let verbose = ChecksumVerbose::new(status, quiet, warn);
        let opts = ChecksumValidateOptions {
            ignore_missing,
            strict,
            verbose,
        };

        return perform_checksum_validation(files, algo_cli, length, opts);
    }

    // Not --check

    // Print hardware debug info if requested
    if matches.get_flag(options::DEBUG) {
        print_cpu_debug_info();
    }

    // Set the default algorithm to CRC when not '--check'ing.
    let algo_kind = algo_cli.unwrap_or(AlgoKind::Crc);

    let (tag, binary) = handle_tag_text_binary_flags(std::env::args_os())?;

    let algo = SizedAlgoKind::from_unsized(algo_kind, length)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let opts = ChecksumComputeOptions {
        algo_kind: algo,
        output_format: figure_out_output_format(
            algo,
            tag,
            binary,
            matches.get_flag(options::RAW),
            matches.get_flag(options::BASE64),
        ),
        line_ending,
    };

    perform_checksum_computation(opts, files)?;

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("cksum-about"))
        .override_usage(format_usage(&translate!("cksum-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help(translate!("cksum-help-algorithm"))
                .value_name("ALGORITHM")
                .value_parser(SUPPORTED_ALGORITHMS),
        )
        .arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help(translate!("cksum-help-untagged"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::TAG),
        )
        .arg(
            Arg::new(options::TAG)
                .long(options::TAG)
                .help(translate!("cksum-help-tag"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::UNTAGGED),
        )
        .arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .short('l')
                .help(translate!("cksum-help-length"))
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help(translate!("cksum-help-raw"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help(translate!("cksum-help-strict"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help(translate!("cksum-help-check"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help(translate!("cksum-help-base64"))
                .action(ArgAction::SetTrue)
                // Even though this could easily just override an earlier '--raw',
                // GNU cksum does not permit these flags to be combined:
                .conflicts_with(options::RAW),
        )
        .arg(
            Arg::new(options::TEXT)
                .long(options::TEXT)
                .short('t')
                .hide(true)
                .overrides_with(options::BINARY)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BINARY)
                .long(options::BINARY)
                .short('b')
                .hide(true)
                .overrides_with(options::TEXT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help(translate!("cksum-help-warn"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::QUIET]),
        )
        .arg(
            Arg::new(options::STATUS)
                .long("status")
                .help(translate!("cksum-help-status"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::WARN, options::QUIET]),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .help(translate!("cksum-help-quiet"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::WARN, options::STATUS]),
        )
        .arg(
            Arg::new(options::IGNORE_MISSING)
                .long(options::IGNORE_MISSING)
                .help(translate!("cksum-help-ignore-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("cksum-help-zero"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help(translate!("cksum-help-debug"))
                .action(ArgAction::SetTrue),
        )
        .after_help(translate!("cksum-after-help"))
}
