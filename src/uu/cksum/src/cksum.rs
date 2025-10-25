// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command, value_parser};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufReader, Read, Write, stdin, stdout};
use std::iter;
use std::path::Path;
use uucore::checksum::{
    ALGORITHM_OPTIONS_BLAKE2B, ALGORITHM_OPTIONS_BSD, ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_CRC32B, ALGORITHM_OPTIONS_SYSV, ChecksumError, ChecksumOptions,
    ChecksumVerbose, SUPPORTED_ALGORITHMS, calculate_blake2b_length, digest_reader,
    perform_checksum_validation,
};
use uucore::translate;

use uucore::{
    encoding,
    error::{FromIo, UResult, USimpleError},
    format_usage,
    line_ending::LineEnding,
    os_str_as_bytes, show,
    sum::Digest,
};

#[derive(Debug, PartialEq)]
enum OutputFormat {
    Hexadecimal,
    Raw,
    Base64,
}

struct Options {
    algo_name: &'static str,
    digest: Box<dyn Digest + 'static>,
    output_bits: usize,
    tag: bool, // will cover the --untagged option
    length: Option<usize>,
    output_format: OutputFormat,
    asterisk: bool, // if we display an asterisk or not (--binary/--text)
    line_ending: LineEnding,
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of [`OsStr`] which is a bunch of files that are using for calculating checksum
#[allow(clippy::cognitive_complexity)]
fn cksum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let files: Vec<_> = files.collect();
    if options.output_format == OutputFormat::Raw && files.len() > 1 {
        return Err(Box::new(ChecksumError::RawMultipleFiles));
    }

    for filename in files {
        let filename = Path::new(filename);
        let stdin_buf;
        let file_buf;
        let is_stdin = filename == OsStr::new("-");

        if filename.is_dir() {
            show!(USimpleError::new(
                1,
                translate!("cksum-error-is-directory", "file" => filename.display())
            ));
            continue;
        }

        // Handle the file input
        let mut file = BufReader::new(if is_stdin {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else {
            file_buf = match File::open(filename) {
                Ok(file) => file,
                Err(err) => {
                    show!(err.map_err_context(|| filename.to_string_lossy().to_string()));
                    continue;
                }
            };
            Box::new(file_buf) as Box<dyn Read>
        });

        let (sum_hex, sz) =
            digest_reader(&mut options.digest, &mut file, false, options.output_bits)
                .map_err_context(|| translate!("cksum-error-failed-to-read-input"))?;

        let sum = match options.output_format {
            OutputFormat::Raw => {
                let bytes = match options.algo_name {
                    ALGORITHM_OPTIONS_CRC => sum_hex.parse::<u32>().unwrap().to_be_bytes().to_vec(),
                    ALGORITHM_OPTIONS_SYSV | ALGORITHM_OPTIONS_BSD => {
                        sum_hex.parse::<u16>().unwrap().to_be_bytes().to_vec()
                    }
                    _ => hex::decode(sum_hex).unwrap(),
                };
                // Cannot handle multiple files anyway, output immediately.
                stdout().write_all(&bytes)?;
                return Ok(());
            }
            OutputFormat::Hexadecimal => sum_hex,
            OutputFormat::Base64 => match options.algo_name {
                ALGORITHM_OPTIONS_CRC
                | ALGORITHM_OPTIONS_CRC32B
                | ALGORITHM_OPTIONS_SYSV
                | ALGORITHM_OPTIONS_BSD => sum_hex,
                _ => encoding::for_cksum::BASE64.encode(&hex::decode(sum_hex).unwrap()),
            },
        };

        // The BSD checksum output is 5 digit integer
        let bsd_width = 5;
        let (before_filename, should_print_filename, after_filename) = match options.algo_name {
            ALGORITHM_OPTIONS_SYSV => (
                format!(
                    "{} {}{}",
                    sum.parse::<u16>().unwrap(),
                    sz.div_ceil(options.output_bits),
                    if is_stdin { "" } else { " " }
                ),
                !is_stdin,
                String::new(),
            ),
            ALGORITHM_OPTIONS_BSD => (
                format!(
                    "{:0bsd_width$} {:bsd_width$}{}",
                    sum.parse::<u16>().unwrap(),
                    sz.div_ceil(options.output_bits),
                    if is_stdin { "" } else { " " }
                ),
                !is_stdin,
                String::new(),
            ),
            ALGORITHM_OPTIONS_CRC | ALGORITHM_OPTIONS_CRC32B => (
                format!("{sum} {sz}{}", if is_stdin { "" } else { " " }),
                !is_stdin,
                String::new(),
            ),
            ALGORITHM_OPTIONS_BLAKE2B if options.tag => {
                (
                    if let Some(length) = options.length {
                        // Multiply by 8 here, as we want to print the length in bits.
                        format!("BLAKE2b-{} (", length * 8)
                    } else {
                        "BLAKE2b (".to_owned()
                    },
                    true,
                    format!(") = {sum}"),
                )
            }
            _ => {
                if options.tag {
                    (
                        format!("{} (", options.algo_name.to_ascii_uppercase()),
                        true,
                        format!(") = {sum}"),
                    )
                } else {
                    let prefix = if options.asterisk { "*" } else { " " };
                    (format!("{sum} {prefix}"), true, String::new())
                }
            }
        };

        print!("{before_filename}");
        if should_print_filename {
            // The filename might not be valid UTF-8, and filename.display() would mangle the names.
            // Therefore, emit the bytes directly to stdout, without any attempt at encoding them.
            let _dropped_result = stdout().write_all(os_str_as_bytes(filename.as_os_str())?);
        }
        print!("{after_filename}{}", options.line_ending);
    }
    Ok(())
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

/***
 * cksum has a bunch of legacy behavior.
 * We handle this in this function to make sure they are self contained
 * and "easier" to understand
 */
fn handle_tag_text_binary_flags<S: AsRef<OsStr>>(
    args: impl Iterator<Item = S>,
) -> UResult<(bool, bool)> {
    let mut tag = true;
    let mut binary = false;

    // --binary, --tag and --untagged are tight together: none of them
    // conflicts with each other but --tag will reset "binary" and set "tag".

    for arg in args {
        let arg = arg.as_ref();
        if arg == "-b" || arg == "--binary" {
            binary = true;
        } else if arg == "--tag" {
            tag = true;
            binary = false;
        } else if arg == "--untagged" {
            tag = false;
        }
    }

    Ok((tag, !tag && binary))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let check = matches.get_flag(options::CHECK);

    let algo_name: &str = match matches.get_one::<String>(options::ALGORITHM) {
        Some(v) => v,
        None => {
            if check {
                // if we are doing a --check, we should not default to crc
                ""
            } else {
                ALGORITHM_OPTIONS_CRC
            }
        }
    };

    let input_length = matches.get_one::<usize>(options::LENGTH);

    let length = match input_length {
        Some(length) => {
            if algo_name == ALGORITHM_OPTIONS_BLAKE2B {
                calculate_blake2b_length(*length)?
            } else if algo_name.starts_with("sha3")
                || algo_name == "shake128"
                || algo_name == "shake256"
            {
                // SHA3 and SHAKE algorithms require --length in bits
                Some(*length)
            } else {
                return Err(USimpleError::new(
                    1,
                    "--length is only supported with --algorithm=blake2b, sha3, shake128, or shake256",
                ));
            }
        }
        None => None,
    };

    if ["bsd", "crc", "sysv", "crc32b"].contains(&algo_name) && check {
        return Err(ChecksumError::AlgorithmNotSupportedWithCheck.into());
    }

    if check {
        let text_flag = matches.get_flag(options::TEXT);
        let binary_flag = matches.get_flag(options::BINARY);
        let strict = matches.get_flag(options::STRICT);
        let status = matches.get_flag(options::STATUS);
        let warn = matches.get_flag(options::WARN);
        let ignore_missing = matches.get_flag(options::IGNORE_MISSING);
        let quiet = matches.get_flag(options::QUIET);
        let tag = matches.get_flag(options::TAG);

        if tag || binary_flag || text_flag {
            return Err(ChecksumError::BinaryTextConflict.into());
        }

        // Determine the appropriate algorithm option to pass
        let algo_option = if algo_name.is_empty() {
            None
        } else {
            Some(algo_name)
        };

        // Execute the checksum validation based on the presence of files or the use of stdin

        let files = matches.get_many::<OsString>(options::FILE).map_or_else(
            || iter::once(OsStr::new("-")).collect::<Vec<_>>(),
            |files| files.map(OsStr::new).collect::<Vec<_>>(),
        );

        let verbose = ChecksumVerbose::new(status, quiet, warn);
        let opts = ChecksumOptions {
            binary: binary_flag,
            ignore_missing,
            strict,
            verbose,
        };

        return perform_checksum_validation(files.iter().copied(), algo_option, length, opts);
    }

    let (tag, asterisk) = handle_tag_text_binary_flags(std::env::args_os())?;

    let algo = uucore::checksum::detect_algo_with_label(algo_name, length, true)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let output_format = if matches.get_flag(options::RAW) {
        OutputFormat::Raw
    } else if matches.get_flag(options::BASE64) {
        OutputFormat::Base64
    } else {
        OutputFormat::Hexadecimal
    };

    let opts = Options {
        algo_name: algo.name,
        digest: (algo.create_fn)(),
        output_bits: algo.bits,
        length,
        tag,
        output_format,
        asterisk,
        line_ending,
    };

    match matches.get_many::<OsString>(options::FILE) {
        Some(files) => cksum(opts, files.map(OsStr::new))?,
        None => cksum(opts, iter::once(OsStr::new("-")))?,
    }

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
                .value_parser(value_parser!(usize))
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
