// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufReader, Read, Write, stdin, stdout};
use std::iter;
use std::path::Path;
use uucore::checksum::{
    ALGORITHM_OPTIONS_BLAKE2B, ALGORITHM_OPTIONS_BSD, ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_CRC32B, ALGORITHM_OPTIONS_SHA2, ALGORITHM_OPTIONS_SHA3,
    ALGORITHM_OPTIONS_SYSV, ChecksumError, ChecksumOptions, ChecksumVerbose, HashAlgorithm,
    LEGACY_ALGORITHMS, SUPPORTED_ALGORITHMS, calculate_blake2b_length_str, detect_algo,
    digest_reader, perform_checksum_validation, sanitize_sha2_sha3_length_str,
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

struct Options {
    algo_name: &'static str,
    digest: Box<dyn Digest + 'static>,
    output_bits: usize,
    length: Option<usize>,
    output_format: OutputFormat,
    line_ending: LineEnding,
}

/// Reading mode used to compute digest.
///
/// On most linux systems, this is irrelevant, as there is no distinction
/// between text and binary files. Refer to GNU's cksum documentation for more
/// information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadingMode {
    Binary,
    Text,
}

impl ReadingMode {
    #[inline]
    fn as_char(&self) -> char {
        match self {
            Self::Binary => '*',
            Self::Text => ' ',
        }
    }
}

/// Whether to write the digest as hexadecimal or encoded in base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DigestFormat {
    Hexadecimal,
    Base64,
}

impl DigestFormat {
    #[inline]
    fn is_base64(&self) -> bool {
        *self == Self::Base64
    }
}

/// Holds the representation that shall be used for printing a checksum line
#[derive(Debug, PartialEq, Eq)]
enum OutputFormat {
    /// Raw digest
    Raw,

    /// Selected for older algorithms which had their custom formatting
    ///
    /// Default for crc, sysv, bsd
    Legacy,

    /// `$ALGO_NAME ($FILENAME) = $DIGEST`
    Tagged(DigestFormat),

    /// '$DIGEST $FLAG$FILENAME'
    /// where 'flag' depends on the reading mode
    ///
    /// Default for standalone checksum utilities
    Untagged(DigestFormat, ReadingMode),
}

impl OutputFormat {
    #[inline]
    fn is_raw(&self) -> bool {
        *self == Self::Raw
    }
}

fn print_legacy_checksum(
    options: &Options,
    filename: &OsStr,
    sum: &str,
    size: usize,
) -> UResult<()> {
    debug_assert!(LEGACY_ALGORITHMS.contains(&options.algo_name));

    // Print the sum
    match options.algo_name {
        ALGORITHM_OPTIONS_SYSV => print!(
            "{} {}",
            sum.parse::<u16>().unwrap(),
            size.div_ceil(options.output_bits),
        ),
        ALGORITHM_OPTIONS_BSD => {
            // The BSD checksum output is 5 digit integer
            let bsd_width = 5;
            print!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                size.div_ceil(options.output_bits),
            );
        }
        ALGORITHM_OPTIONS_CRC | ALGORITHM_OPTIONS_CRC32B => {
            print!("{sum} {size}");
        }
        _ => unreachable!("Not a legacy algorithm"),
    }

    // Print the filename after a space if not stdin
    if filename != "-" {
        print!(" ");
        let _dropped_result = stdout().write_all(os_str_as_bytes(filename)?);
    }

    Ok(())
}

fn print_tagged_checksum(options: &Options, filename: &OsStr, sum: &String) -> UResult<()> {
    // Print algo name and opening parenthesis.
    print!(
        "{} (",
        match (options.algo_name, options.length) {
            // Multiply the length by 8, as we want to print the length in bits.
            (ALGORITHM_OPTIONS_BLAKE2B, Some(l)) => format!("BLAKE2b-{}", l * 8),
            (ALGORITHM_OPTIONS_BLAKE2B, None) => "BLAKE2b".into(),
            (name, _) => name.to_ascii_uppercase(),
        }
    );

    // Print filename
    let _dropped_result = stdout().write_all(os_str_as_bytes(filename)?);

    // Print closing parenthesis and sum
    print!(") = {sum}");

    Ok(())
}

fn print_untagged_checksum(
    filename: &OsStr,
    sum: &String,
    reading_mode: ReadingMode,
) -> UResult<()> {
    // Print checksum and reading mode flag
    print!("{sum} {}", reading_mode.as_char());

    // Print filename
    let _dropped_result = stdout().write_all(os_str_as_bytes(filename)?);

    Ok(())
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of [`OsStr`] which is a bunch of files that are using for calculating checksum
fn cksum<'a, I>(mut options: Options, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut files = files.peekable();

    while let Some(filename) = files.next() {
        // Check that in raw mode, we are not provided with several files.
        if options.output_format.is_raw() && files.peek().is_some() {
            return Err(Box::new(ChecksumError::RawMultipleFiles));
        }

        let filepath = Path::new(filename);
        let stdin_buf;
        let file_buf;
        if filepath.is_dir() {
            show!(USimpleError::new(
                1,
                translate!("cksum-error-is-directory", "file" => filepath.display())
            ));
            continue;
        }

        // Handle the file input
        let mut file = BufReader::new(if filename == "-" {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else {
            file_buf = match File::open(filepath) {
                Ok(file) => file,
                Err(err) => {
                    show!(err.map_err_context(|| filepath.to_string_lossy().to_string()));
                    continue;
                }
            };
            Box::new(file_buf) as Box<dyn Read>
        });

        let (sum_hex, sz) =
            digest_reader(&mut options.digest, &mut file, false, options.output_bits)
                .map_err_context(|| translate!("cksum-error-failed-to-read-input"))?;

        // Encodes the sum if df is Base64, leaves as-is otherwise.
        let encode_sum = |sum: String, df: DigestFormat| {
            if df.is_base64() {
                encoding::for_cksum::BASE64.encode(&hex::decode(sum).unwrap())
            } else {
                sum
            }
        };

        match options.output_format {
            OutputFormat::Raw => {
                let bytes = match options.algo_name {
                    ALGORITHM_OPTIONS_CRC | ALGORITHM_OPTIONS_CRC32B => {
                        sum_hex.parse::<u32>().unwrap().to_be_bytes().to_vec()
                    }
                    ALGORITHM_OPTIONS_SYSV | ALGORITHM_OPTIONS_BSD => {
                        sum_hex.parse::<u16>().unwrap().to_be_bytes().to_vec()
                    }
                    _ => hex::decode(sum_hex).unwrap(),
                };
                // Cannot handle multiple files anyway, output immediately.
                stdout().write_all(&bytes)?;
                return Ok(());
            }
            OutputFormat::Legacy => {
                print_legacy_checksum(&options, filename, &sum_hex, sz)?;
            }
            OutputFormat::Tagged(digest_format) => {
                print_tagged_checksum(&options, filename, &encode_sum(sum_hex, digest_format))?;
            }
            OutputFormat::Untagged(digest_format, reading_mode) => {
                print_untagged_checksum(
                    filename,
                    &encode_sum(sum_hex, digest_format),
                    reading_mode,
                )?;
            }
        }

        print!("{}", options.line_ending);
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

/// Use already-processed arguments to decide the output format.
fn figure_out_output_format(
    algo: &HashAlgorithm,
    tag: bool,
    binary: bool,
    raw: bool,
    base64: bool,
) -> OutputFormat {
    // Raw output format takes precedence over anything else.
    if raw {
        return OutputFormat::Raw;
    }

    // Then, if the algo is legacy, takes precedence over the rest
    if LEGACY_ALGORITHMS.contains(&algo.name) {
        return OutputFormat::Legacy;
    }

    let digest_format = if base64 {
        DigestFormat::Base64
    } else {
        DigestFormat::Hexadecimal
    };

    // After that, decide between tagged and untagged output
    if tag {
        OutputFormat::Tagged(digest_format)
    } else {
        let reading_mode = if binary {
            ReadingMode::Binary
        } else {
            ReadingMode::Text
        };
        OutputFormat::Untagged(digest_format, reading_mode)
    }
}

/// Sanitize the `--length` argument depending on `--algorithm` and `--length`.
fn maybe_sanitize_length(
    algo_cli: Option<&str>,
    input_length: Option<&str>,
) -> UResult<Option<usize>> {
    match (algo_cli, input_length) {
        // No provided length is not a problem so far.
        (_, None) => Ok(None),

        // For SHA2 and SHA3, if a length is provided, ensure it is correct.
        (Some(algo @ (ALGORITHM_OPTIONS_SHA2 | ALGORITHM_OPTIONS_SHA3)), Some(s_len)) => {
            sanitize_sha2_sha3_length_str(algo, s_len).map(Some)
        }

        // For BLAKE2b, if a length is provided, validate it.
        (Some(ALGORITHM_OPTIONS_BLAKE2B), Some(len)) => calculate_blake2b_length_str(len),

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

    let algo_cli = matches
        .get_one::<String>(options::ALGORITHM)
        .map(String::as_str);

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
        if algo_cli.is_some_and(|algo_name| LEGACY_ALGORITHMS.contains(&algo_name)) {
            return Err(ChecksumError::AlgorithmNotSupportedWithCheck.into());
        }

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

        // Execute the checksum validation based on the presence of files or the use of stdin

        let verbose = ChecksumVerbose::new(status, quiet, warn);
        let opts = ChecksumOptions {
            binary: binary_flag,
            ignore_missing,
            strict,
            verbose,
        };

        return perform_checksum_validation(files, algo_cli, length, opts);
    }

    // Not --check

    // Set the default algorithm to CRC when not '--check'ing.
    let algo_name = algo_cli.unwrap_or(ALGORITHM_OPTIONS_CRC);

    let (tag, binary) = handle_tag_text_binary_flags(std::env::args_os())?;

    let algo = detect_algo(algo_name, length)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let output_format = figure_out_output_format(
        &algo,
        tag,
        binary,
        matches.get_flag(options::RAW),
        matches.get_flag(options::BASE64),
    );

    let opts = Options {
        algo_name: algo.name,
        digest: (algo.create_fn)(),
        output_bits: algo.bits,
        length,
        output_format,
        line_ending,
    };

    cksum(opts, files)?;

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
        .after_help(translate!("cksum-after-help"))
}
