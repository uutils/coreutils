// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo

use std::ffi::OsString;

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command, ValueHint};

use uucore::checksum::compute::{
    ChecksumComputeOptions, OutputFormat, perform_checksum_computation,
};
use uucore::checksum::validate::{self, ChecksumValidateOptions, ChecksumVerbose};
use uucore::checksum::{AlgoKind, ChecksumError, SizedAlgoKind};
use uucore::error::UResult;
use uucore::line_ending::LineEnding;
use uucore::{crate_version, format_usage, localized_help_template, util_name};

mod cli;
pub use cli::ChecksumCommand;
pub use cli::options;

/// Entrypoint for standalone checksums accepting the `--length` argument
///
/// Note: Ideally, we wouldn't require a `cmd` to be passed to the function,
/// but for localization purposes, the standalone binaries must declare their
/// command (with about and usage) themselves, otherwise calling --help from
/// the multicall binary results in an unformatted output.
pub fn standalone_with_length_main(
    algo: AlgoKind,
    cmd: Command,
    args: impl uucore::Args,
    validate_len: fn(&str) -> UResult<Option<usize>>,
) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(cmd, args)?;
    let algo = Some(algo);

    let length = matches
        .get_one::<String>(options::LENGTH)
        .map(String::as_str)
        .map(validate_len)
        .transpose()?
        .flatten();

    let format = OutputFormat::from_standalone(std::env::args_os());

    checksum_main(algo, length, matches, format?)
}

/// Entrypoint for standalone checksums *NOT* accepting the `--length` argument
pub fn standalone_main(algo: AlgoKind, cmd: Command, args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(cmd, args)?;
    let algo = Some(algo);

    let format = OutputFormat::from_standalone(std::env::args_os());

    checksum_main(algo, None, matches, format?)
}

/// Base command processing for all the checksum executables.
pub fn default_checksum_app(about: String, usage: String) -> Command {
    Command::new(util_name())
        .version(crate_version!())
        .help_template(localized_help_template(util_name()))
        .about(about)
        .override_usage(format_usage(&usage))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .default_value("-")
                .hide_default_value(true)
                .value_hint(ValueHint::FilePath),
        )
}

/// Command processing for standalone checksums accepting the `--length`
/// argument
pub fn standalone_checksum_app_with_length(about: String, usage: String) -> Command {
    default_checksum_app(about, usage)
        .with_binary(/* needs_untagged */ false)
        .with_check_and_opts()
        .with_length()
        .with_tag(false)
        .with_text(/* needs_untagged */ false, true)
        .with_zero()
}

/// Command processing for standalone checksums *NOT* accepting the `--length`
/// argument
pub fn standalone_checksum_app(about: String, usage: String) -> Command {
    default_checksum_app(about, usage)
        .with_binary(/* needs_untagged */ false)
        .with_check_and_opts()
        .with_tag(false)
        .with_text(/* needs_untagged */ false, true)
        .with_zero()
}

/// This is the common entrypoint to all checksum utils. Performs some
/// validation on arguments and proceeds in computing or checking mode.
pub fn checksum_main(
    algo: Option<AlgoKind>,
    length: Option<usize>,
    matches: ArgMatches,
    output_format: OutputFormat,
) -> UResult<()> {
    let check = matches.get_flag(options::CHECK);

    let ignore_missing = matches.get_flag(options::IGNORE_MISSING);
    let warn = matches.get_flag(options::WARN);
    let quiet = matches.get_flag(options::QUIET);
    let strict = matches.get_flag(options::STRICT);
    let status = matches.get_flag(options::STATUS);

    // clap provides the default value -. So we unwrap() safety.
    let files = matches
        .get_many::<OsString>(options::FILE)
        .unwrap()
        .map(|s| s.as_os_str());

    if check {
        // cksum does not support '--check'ing legacy algorithms
        if algo.is_some_and(AlgoKind::is_legacy) {
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

        return validate::perform_checksum_validation(files, algo, length, opts);
    }

    // Not --check

    // Set the default algorithm to CRC when not '--check'ing.
    let algo_kind = algo.unwrap_or(AlgoKind::Crc);

    let algo = SizedAlgoKind::from_unsized(algo_kind, length)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let opts = ChecksumComputeOptions {
        algo_kind: algo,
        output_format,
        line_ending,
    };

    perform_checksum_computation(opts, files)?;

    Ok(())
}
