use std::ffi::{OsStr, OsString};

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command, ValueHint};

use crate::checksum::compute::{
    ChecksumComputeOptions, OutputFormat, perform_checksum_computation,
};
use crate::checksum::validate::{ChecksumValidateOptions, ChecksumVerbose};
use crate::checksum::{AlgoKind, ChecksumError, SUPPORTED_ALGORITHMS, SizedAlgoKind};
use crate::error::UResult;
use crate::line_ending::LineEnding;
use crate::{crate_version, format_usage, localized_help_template, translate, util_name};

pub mod options {
    // cksum-specific
    pub const ALGORITHM: &str = "algorithm";
    pub const DEBUG: &str = "debug";

    pub const FILE: &str = "file";

    pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    pub const RAW: &str = "raw";
    pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const ZERO: &str = "zero";

    // check-specific
    pub const STRICT: &str = "strict";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const IGNORE_MISSING: &str = "ignore-missing";
    pub const QUIET: &str = "quiet";
}

pub trait ChecksumCommand {
    fn with_algo(self) -> Self;

    fn with_length(self) -> Self;

    fn with_check(self) -> Self;

    fn with_binary(self) -> Self;

    fn with_text(self, default: bool) -> Self;

    fn with_tag(self, default: bool) -> Self;

    fn with_untagged(self) -> Self;

    fn with_raw(self) -> Self;

    fn with_base64(self) -> Self;

    fn with_zero(self) -> Self;

    fn with_debug(self) -> Self;
}

impl ChecksumCommand for Command {
    fn with_algo(self) -> Self {
        self.arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help(translate!("checksum-help-algorithm"))
                .value_name("ALGORITHM")
                .value_parser(SUPPORTED_ALGORITHMS),
        )
    }

    fn with_length(self) -> Self {
        self.arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .short('l')
                .help(translate!("checksum-help-length"))
                .action(ArgAction::Set),
        )
    }

    fn with_check(self) -> Self {
        self.arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help(translate!("checksum-help-check"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help(translate!("checksum-help-warn"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::QUIET]),
        )
        .arg(
            Arg::new(options::STATUS)
                .long("status")
                .help(translate!("checksum-help-status"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::WARN, options::QUIET]),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .help(translate!("checksum-help-quiet"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::WARN, options::STATUS]),
        )
        .arg(
            Arg::new(options::IGNORE_MISSING)
                .long(options::IGNORE_MISSING)
                .help(translate!("checksum-help-ignore-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help(translate!("checksum-help-strict"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_binary(self) -> Self {
        self.arg(
            Arg::new(options::BINARY)
                .long(options::BINARY)
                .short('b')
                .hide(true)
                .overrides_with(options::TEXT)
                .action(ArgAction::SetTrue),
        )
    }

    fn with_text(self, default: bool) -> Self {
        let mut arg = Arg::new(options::TEXT)
            .long(options::TEXT)
            .short('t')
            .action(ArgAction::SetTrue);
        if default {
            arg = arg.help(translate!("checksum-help-text"));
        } else {
            arg = arg.hide(true);
        }
        self.arg(arg)
    }

    fn with_tag(self, default: bool) -> Self {
        self.arg(
            Arg::new(options::TAG)
                .long(options::TAG)
                .help(if default {
                    translate!("checksum-help-tag-default")
                } else {
                    translate!("checksum-help-tag")
                })
                .action(ArgAction::SetTrue),
        )
    }

    fn with_untagged(self) -> Self {
        self.arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help(translate!("checksum-help-untagged"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::TAG),
        )
    }

    fn with_raw(self) -> Self {
        self.arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help(translate!("checksum-help-raw"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_base64(self) -> Self {
        self.arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help(translate!("checksum-help-base64"))
                .action(ArgAction::SetTrue)
                // Even though this could easily just override an earlier '--raw',
                // GNU cksum does not permit these flags to be combined:
                .conflicts_with(options::RAW),
        )
    }

    fn with_zero(self) -> Self {
        self.arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("checksum-help-zero"))
                .action(ArgAction::SetTrue),
        )
    }

    fn with_debug(self) -> Self {
        self.arg(
            Arg::new(options::DEBUG)
                .long(options::DEBUG)
                .help(translate!("checksum-help-debug"))
                .action(ArgAction::SetTrue),
        )
    }
}

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
                .value_hint(ValueHint::FilePath),
        )
}

pub fn standalone_checksum_app(about: String, usage: String) -> Command {
    default_checksum_app(about, usage)
        .with_binary()
        .with_check()
        .with_tag(false)
        .with_text(true)
        .with_zero()
}

pub fn standalone_checksum_app_with_length(about: String, usage: String) -> Command {
    default_checksum_app(about, usage)
        .with_binary()
        .with_check()
        .with_length()
        .with_tag(false)
        .with_text(true)
        .with_zero()
}

pub fn checksum_main(
    algo: Option<AlgoKind>,
    length: Option<usize>,
    matches: ArgMatches,
    output_format: OutputFormat,
) -> UResult<()> {
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

    let files = matches.get_many::<OsString>(options::FILE).map_or_else(
        // No files given, read from stdin.
        || Box::new(std::iter::once(OsStr::new("-"))) as Box<dyn Iterator<Item = &OsStr>>,
        // At least one file given, read from them.
        |files| Box::new(files.map(OsStr::new)) as Box<dyn Iterator<Item = &OsStr>>,
    );

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

        return super::validate::perform_checksum_validation(files, algo, length, opts);
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
