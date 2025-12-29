// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo, algoname, bitlen, regexes, nread

use std::ffi::{OsStr, OsString};
use std::iter;
use std::path::Path;

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command};

use uucore::checksum::compute::{
    ChecksumComputeOptions, figure_out_output_format, perform_checksum_computation,
};
use uucore::checksum::validate::{
    ChecksumValidateOptions, ChecksumVerbose, perform_checksum_validation,
};
use uucore::checksum::{
    AlgoKind, ChecksumError, SizedAlgoKind, calculate_blake2b_length_str,
    sanitize_sha2_sha3_length_str,
};
use uucore::error::UResult;
use uucore::line_ending::LineEnding;
use uucore::{format_usage, translate};

const NAME: &str = "hashsum";

/// Creates a hasher instance based on the command-line flags.
///
/// # Arguments
///
/// * `matches` - A reference to the `ArgMatches` object containing the command-line arguments.
///
/// # Returns
///
/// Returns a [`UResult`] of a tuple containing the algorithm name, the hasher instance, and
/// the output length in bits or an Err if multiple hash algorithms are specified or if a
/// required flag is missing.
#[allow(clippy::cognitive_complexity)]
fn create_algorithm_from_flags(matches: &ArgMatches) -> UResult<(AlgoKind, Option<usize>)> {
    let mut alg: Option<(AlgoKind, Option<usize>)> = None;

    let mut set_or_err = |new_alg: (AlgoKind, Option<usize>)| -> UResult<()> {
        if alg.is_some() {
            return Err(ChecksumError::CombineMultipleAlgorithms.into());
        }
        alg = Some(new_alg);
        Ok(())
    };

    if matches.get_flag("md5") {
        set_or_err((AlgoKind::Md5, None))?;
    }
    if matches.get_flag("sha1") {
        set_or_err((AlgoKind::Sha1, None))?;
    }
    if matches.get_flag("sha224") {
        set_or_err((AlgoKind::Sha224, None))?;
    }
    if matches.get_flag("sha256") {
        set_or_err((AlgoKind::Sha256, None))?;
    }
    if matches.get_flag("sha384") {
        set_or_err((AlgoKind::Sha384, None))?;
    }
    if matches.get_flag("sha512") {
        set_or_err((AlgoKind::Sha512, None))?;
    }
    if matches.get_flag("b2sum") {
        set_or_err((AlgoKind::Blake2b, None))?;
    }
    if matches.get_flag("b3sum") {
        set_or_err((AlgoKind::Blake3, None))?;
    }
    if matches.get_flag("sha3") {
        match matches.get_one::<String>(options::LENGTH) {
            Some(len) => set_or_err((
                AlgoKind::Sha3,
                Some(sanitize_sha2_sha3_length_str(AlgoKind::Sha3, len)?),
            ))?,
            None => return Err(ChecksumError::LengthRequired("SHA3".into()).into()),
        }
    }
    if matches.get_flag("sha3-224") {
        set_or_err((AlgoKind::Sha3, Some(224)))?;
    }
    if matches.get_flag("sha3-256") {
        set_or_err((AlgoKind::Sha3, Some(256)))?;
    }
    if matches.get_flag("sha3-384") {
        set_or_err((AlgoKind::Sha3, Some(384)))?;
    }
    if matches.get_flag("sha3-512") {
        set_or_err((AlgoKind::Sha3, Some(512)))?;
    }
    if matches.get_flag("shake128") {
        set_or_err((AlgoKind::Shake128, Some(128)))?;
    }
    if matches.get_flag("shake256") {
        set_or_err((AlgoKind::Shake256, Some(256)))?;
    }

    if alg.is_none() {
        return Err(ChecksumError::NeedAlgorithmToHash.into());
    }

    Ok(alg.unwrap())
}

#[uucore::main]
pub fn uumain(mut args: impl uucore::Args) -> UResult<()> {
    // if there is no program name for some reason, default to "hashsum"
    let program = args.next().unwrap_or_else(|| OsString::from(NAME));
    let binary_name = Path::new(&program)
        .file_stem()
        .unwrap_or_else(|| OsStr::new(NAME))
        .to_string_lossy();

    let args = iter::once(program.clone()).chain(args);

    // Default binary in Windows, text mode otherwise
    let binary_flag_default = cfg!(windows);

    let (command, is_hashsum_bin) = uu_app(&binary_name);

    // FIXME: this should use try_get_matches_from() and crash!(), but at the moment that just
    //        causes "error: " to be printed twice (once from crash!() and once from clap).  With
    //        the current setup, the name of the utility is not printed, but I think this is at
    //        least somewhat better from a user's perspective.
    let matches = uucore::clap_localization::handle_clap_result(command, args)?;

    let length: Option<usize> = if binary_name == "b2sum" {
        if let Some(len) = matches.get_one::<String>(options::LENGTH) {
            calculate_blake2b_length_str(len)?
        } else {
            None
        }
    } else {
        None
    };

    let (algo_kind, length) = if is_hashsum_bin {
        create_algorithm_from_flags(&matches)?
    } else {
        (AlgoKind::from_bin_name(&binary_name)?, length)
    };

    let binary = if matches.get_flag("binary") {
        true
    } else if matches.get_flag("text") {
        false
    } else {
        binary_flag_default
    };
    let check = matches.get_flag("check");

    let check_flag = |flag| match (check, matches.get_flag(flag)) {
        (_, false) => Ok(false),
        (true, true) => Ok(true),
        (false, true) => Err(ChecksumError::CheckOnlyFlag(flag.into())),
    };

    // Each of the following flags are only expected in --check mode.
    // If we encounter them otherwise, end with an error.
    let ignore_missing = check_flag("ignore-missing")?;
    let warn = check_flag("warn")?;
    let quiet = check_flag("quiet")?;
    let strict = check_flag("strict")?;
    let status = check_flag("status")?;

    let files = matches.get_many::<OsString>(options::FILE).map_or_else(
        // No files given, read from stdin.
        || Box::new(iter::once(OsStr::new("-"))) as Box<dyn Iterator<Item = &OsStr>>,
        // At least one file given, read from them.
        |files| Box::new(files.map(OsStr::new)) as Box<dyn Iterator<Item = &OsStr>>,
    );

    if check {
        // on Windows, allow --binary/--text to be used with --check
        // and keep the behavior of defaulting to binary
        #[cfg(not(windows))]
        {
            let text_flag = matches.get_flag("text");
            let binary_flag = matches.get_flag("binary");

            if binary_flag || text_flag {
                return Err(ChecksumError::BinaryTextConflict.into());
            }
        }

        let verbose = ChecksumVerbose::new(status, quiet, warn);

        let opts = ChecksumValidateOptions {
            ignore_missing,
            strict,
            verbose,
        };

        // Execute the checksum validation
        return perform_checksum_validation(files, Some(algo_kind), length, opts);
    }

    let algo = SizedAlgoKind::from_unsized(algo_kind, length)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag("zero"));

    let opts = ChecksumComputeOptions {
        algo_kind: algo,
        output_format: figure_out_output_format(
            algo,
            matches.get_flag(options::TAG),
            binary,
            /* raw */ false,
            /* base64: */ false,
        ),
        line_ending,
    };

    // Show the hashsum of the input
    perform_checksum_computation(opts, files)
}

mod options {
    //pub const ALGORITHM: &str = "algorithm";
    pub const FILE: &str = "file";
    //pub const UNTAGGED: &str = "untagged";
    pub const TAG: &str = "tag";
    pub const LENGTH: &str = "length";
    //pub const RAW: &str = "raw";
    //pub const BASE64: &str = "base64";
    pub const CHECK: &str = "check";
    pub const STRICT: &str = "strict";
    pub const TEXT: &str = "text";
    pub const BINARY: &str = "binary";
    pub const STATUS: &str = "status";
    pub const WARN: &str = "warn";
    pub const QUIET: &str = "quiet";
}

pub fn uu_app_common() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("hashsum-about"))
        .override_usage(format_usage(&translate!("hashsum-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::BINARY)
                .short('b')
                .long("binary")
                .help({
                    #[cfg(windows)]
                    {
                        translate!("hashsum-help-binary-windows")
                    }
                    #[cfg(not(windows))]
                    {
                        translate!("hashsum-help-binary-other")
                    }
                })
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long("check")
                .help(translate!("hashsum-help-check"))
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::TAG)
                .long("tag")
                .help(translate!("hashsum-help-tag"))
                .action(ArgAction::SetTrue)
                .conflicts_with("text"),
        )
        .arg(
            Arg::new(options::TEXT)
                .short('t')
                .long("text")
                .help({
                    #[cfg(windows)]
                    {
                        translate!("hashsum-help-text-windows")
                    }
                    #[cfg(not(windows))]
                    {
                        translate!("hashsum-help-text-other")
                    }
                })
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .help(translate!("hashsum-help-quiet"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::STATUS, options::WARN]),
        )
        .arg(
            Arg::new(options::STATUS)
                .short('s')
                .long("status")
                .help(translate!("hashsum-help-status"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::QUIET, options::WARN]),
        )
        .arg(
            Arg::new(options::STRICT)
                .long("strict")
                .help(translate!("hashsum-help-strict"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore-missing")
                .long("ignore-missing")
                .help(translate!("hashsum-help-ignore-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WARN)
                .short('w')
                .long("warn")
                .help(translate!("hashsum-help-warn"))
                .action(ArgAction::SetTrue)
                .overrides_with_all([options::QUIET, options::STATUS]),
        )
        .arg(
            Arg::new("zero")
                .short('z')
                .long("zero")
                .help(translate!("hashsum-help-zero"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(ArgAction::Append)
                .value_name(options::FILE)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string()),
        )
}

pub fn uu_app_length() -> Command {
    uu_app_opt_length(uu_app_common())
}

fn uu_app_opt_length(command: Command) -> Command {
    command.arg(
        Arg::new(options::LENGTH)
            .long(options::LENGTH)
            .short('l')
            .help(translate!("hashsum-help-length"))
            .overrides_with(options::LENGTH)
            .action(ArgAction::Set),
    )
}

pub fn uu_app_custom() -> Command {
    let mut command = uu_app_opt_length(uu_app_common());
    let algorithms = &[
        ("md5", translate!("hashsum-help-md5")),
        ("sha1", translate!("hashsum-help-sha1")),
        ("sha224", translate!("hashsum-help-sha224")),
        ("sha256", translate!("hashsum-help-sha256")),
        ("sha384", translate!("hashsum-help-sha384")),
        ("sha512", translate!("hashsum-help-sha512")),
        ("sha3", translate!("hashsum-help-sha3")),
        ("sha3-224", translate!("hashsum-help-sha3-224")),
        ("sha3-256", translate!("hashsum-help-sha3-256")),
        ("sha3-384", translate!("hashsum-help-sha3-384")),
        ("sha3-512", translate!("hashsum-help-sha3-512")),
        ("shake128", translate!("hashsum-help-shake128")),
        ("shake256", translate!("hashsum-help-shake256")),
        ("b2sum", translate!("hashsum-help-b2sum")),
        ("b3sum", translate!("hashsum-help-b3sum")),
    ];

    for (name, desc) in algorithms {
        command = command.arg(
            Arg::new(*name)
                .long(name)
                .help(desc)
                .action(ArgAction::SetTrue),
        );
    }
    command
}

/// hashsum is handled differently in build.rs
/// therefore, this is different from other utilities.
fn uu_app(binary_name: &str) -> (Command, bool) {
    let (command, is_hashsum_bin) = match binary_name {
        // These all support the same options.
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
            (uu_app_common(), false)
        }
        // b2sum supports the md5sum options plus -l/--length.
        "b2sum" => (uu_app_length(), false),
        // We're probably just being called as `hashsum`, so give them everything.
        _ => (uu_app_custom(), true),
    };

    // If not called as generic hashsum, override the command name and usage
    let command = if is_hashsum_bin {
        command
    } else {
        let usage = translate!("hashsum-usage-specific", "utility_name" => binary_name);
        command
            .help_template(uucore::localized_help_template(binary_name))
            .override_usage(format_usage(&usage))
    };

    (command, is_hashsum_bin)
}
