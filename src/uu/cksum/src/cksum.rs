// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::builder::ValueParser;
use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, stdin, stdout, BufReader, Read, Write};
use std::iter;
use std::path::Path;
use uucore::checksum::{
    calculate_blake2b_length, detect_algo, digest_reader, perform_checksum_validation,
    ChecksumError, ALGORITHM_OPTIONS_BLAKE2B, ALGORITHM_OPTIONS_BSD, ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_SYSV, SUPPORTED_ALGORITHMS,
};
use uucore::{
    encoding,
    error::{FromIo, UResult, USimpleError},
    format_usage, help_about, help_section, help_usage, os_str_as_bytes, show,
    sum::{div_ceil, Digest},
};

const USAGE: &str = help_usage!("cksum.md");
const ABOUT: &str = help_about!("cksum.md");
const AFTER_HELP: &str = help_section!("after help", "cksum.md");

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
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of OsStr which is a bunch of files that are using for calculating checksum
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
        let not_file = filename == OsStr::new("-");

        // Handle the file input
        let mut file = BufReader::new(if not_file {
            stdin_buf = stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else if filename.is_dir() {
            Box::new(BufReader::new(io::empty())) as Box<dyn Read>
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

        if filename.is_dir() {
            show!(USimpleError::new(
                1,
                format!("{}: Is a directory", filename.display())
            ));
            continue;
        }

        let (sum_hex, sz) =
            digest_reader(&mut options.digest, &mut file, false, options.output_bits)
                .map_err_context(|| "failed to read input".to_string())?;

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
                ALGORITHM_OPTIONS_CRC | ALGORITHM_OPTIONS_SYSV | ALGORITHM_OPTIONS_BSD => sum_hex,
                _ => encoding::encode(encoding::Format::Base64, &hex::decode(sum_hex).unwrap())
                    .unwrap(),
            },
        };
        // The BSD checksum output is 5 digit integer
        let bsd_width = 5;
        let (before_filename, should_print_filename, after_filename) = match options.algo_name {
            ALGORITHM_OPTIONS_SYSV => (
                format!(
                    "{} {}{}",
                    sum.parse::<u16>().unwrap(),
                    div_ceil(sz, options.output_bits),
                    if not_file { "" } else { " " }
                ),
                !not_file,
                String::new(),
            ),
            ALGORITHM_OPTIONS_BSD => (
                format!(
                    "{:0bsd_width$} {:bsd_width$}{}",
                    sum.parse::<u16>().unwrap(),
                    div_ceil(sz, options.output_bits),
                    if not_file { "" } else { " " }
                ),
                !not_file,
                String::new(),
            ),
            ALGORITHM_OPTIONS_CRC => (
                format!("{sum} {sz}{}", if not_file { "" } else { " " }),
                !not_file,
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
        print!("{}", before_filename);
        if should_print_filename {
            // The filename might not be valid UTF-8, and filename.display() would mangle the names.
            // Therefore, emit the bytes directly to stdout, without any attempt at encoding them.
            let _dropped_result = stdout().write_all(os_str_as_bytes(filename.as_os_str())?);
        }
        println!("{}", after_filename);
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
}

/// Determines whether to prompt an asterisk (*) in the output.
///
/// This function checks the `tag`, `binary`, and `had_reset` flags and returns a boolean
/// indicating whether to prompt an asterisk (*) in the output.
/// It relies on the overrides provided by clap (i.e., `--binary` overrides `--text` and vice versa).
/// Same for `--tag` and `--untagged`.
fn prompt_asterisk(tag: bool, binary: bool, had_reset: bool) -> bool {
    if tag {
        return false;
    }
    if had_reset {
        return false;
    }
    binary
}

/**
 * Determine if we had a reset.
 * This is basically a hack to support the behavior of cksum
 * when we have the following arguments:
 * --binary --tag --untagged
 * Don't do it with clap because if it struggling with the --overrides_with
 * marking the value as set even if not present
 */
fn had_reset(args: &[OsString]) -> bool {
    // Indices where "--binary" or "-b", "--tag", and "--untagged" are found
    let binary_index = args.iter().position(|x| x == "--binary" || x == "-b");
    let tag_index = args.iter().position(|x| x == "--tag");
    let untagged_index = args.iter().position(|x| x == "--untagged");

    // Check if all arguments are present and in the correct order
    match (binary_index, tag_index, untagged_index) {
        (Some(b), Some(t), Some(u)) => b < t && t < u,
        _ => false,
    }
}

/***
 * cksum has a bunch of legacy behavior.
 * We handle this in this function to make sure they are self contained
 * and "easier" to understand
 */
fn handle_tag_text_binary_flags(matches: &clap::ArgMatches) -> UResult<(bool, bool)> {
    let untagged: bool = matches.get_flag(options::UNTAGGED);
    let tag: bool = matches.get_flag(options::TAG);
    let tag: bool = tag || !untagged;

    let binary_flag: bool = matches.get_flag(options::BINARY);

    let args: Vec<OsString> = std::env::args_os().collect();
    let had_reset = had_reset(&args);

    let asterisk: bool = prompt_asterisk(tag, binary_flag, had_reset);

    Ok((tag, asterisk))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

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

    if ["bsd", "crc", "sysv"].contains(&algo_name) && check {
        return Err(ChecksumError::AlgorithmNotSupportedWithCheck.into());
    }

    let input_length = matches.get_one::<usize>(options::LENGTH);

    let length = match input_length {
        Some(length) => {
            if algo_name == ALGORITHM_OPTIONS_BLAKE2B {
                calculate_blake2b_length(*length)?
            } else {
                return Err(ChecksumError::LengthOnlyForBlake2b.into());
            }
        }
        None => None,
    };

    if check {
        let text_flag = matches.get_flag(options::TEXT);
        let binary_flag = matches.get_flag(options::BINARY);
        let strict = matches.get_flag(options::STRICT);
        let status = matches.get_flag(options::STATUS);
        let warn = matches.get_flag(options::WARN);
        let ignore_missing = matches.get_flag(options::IGNORE_MISSING);
        let quiet = matches.get_flag(options::QUIET);

        if binary_flag || text_flag {
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
        return perform_checksum_validation(
            files.iter().copied(),
            strict,
            status,
            warn,
            binary_flag,
            ignore_missing,
            quiet,
            algo_option,
            length,
        );
    }

    let (tag, asterisk) = handle_tag_text_binary_flags(&matches)?;

    let algo = detect_algo(algo_name, length)?;

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
    };

    match matches.get_many::<OsString>(options::FILE) {
        Some(files) => cksum(opts, files.map(OsStr::new))?,
        None => cksum(opts, iter::once(OsStr::new("-")))?,
    };

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(clap::ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help("select the digest type to use. See DIGEST below")
                .value_name("ALGORITHM")
                .value_parser(SUPPORTED_ALGORITHMS),
        )
        .arg(
            Arg::new(options::UNTAGGED)
                .long(options::UNTAGGED)
                .help("create a reversed style checksum, without digest type")
                .action(ArgAction::SetTrue)
                .overrides_with(options::TAG),
        )
        .arg(
            Arg::new(options::TAG)
                .long(options::TAG)
                .help("create a BSD style checksum, undo --untagged (default)")
                .action(ArgAction::SetTrue)
                .overrides_with(options::UNTAGGED),
        )
        .arg(
            Arg::new(options::LENGTH)
                .long(options::LENGTH)
                .value_parser(value_parser!(usize))
                .short('l')
                .help(
                    "digest length in bits; must not exceed the max for the blake2 algorithm \
                    and must be a multiple of 8",
                )
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::RAW)
                .long(options::RAW)
                .help("emit a raw binary digest, not hexadecimal")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STRICT)
                .long(options::STRICT)
                .help("exit non-zero for improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK)
                .short('c')
                .long(options::CHECK)
                .help("read hashsums from the FILEs and check them")
                .action(ArgAction::SetTrue)
                .conflicts_with("tag"),
        )
        .arg(
            Arg::new(options::BASE64)
                .long(options::BASE64)
                .help("emit a base64 digest, not hexadecimal")
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
                .help("warn about improperly formatted checksum lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::STATUS)
                .long("status")
                .help("don't output anything, status code shows success")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .help("don't print OK for each successfully verified file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_MISSING)
                .long(options::IGNORE_MISSING)
                .help("don't fail or report status for missing files")
                .action(ArgAction::SetTrue),
        )
        .after_help(AFTER_HELP)
}

#[cfg(test)]
mod tests {
    use super::had_reset;
    use crate::calculate_blake2b_length;
    use crate::prompt_asterisk;
    use std::ffi::OsString;

    #[test]
    fn test_had_reset() {
        let args = ["--binary", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(had_reset(&args));

        let args = ["-b", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(had_reset(&args));

        let args = ["-b", "--binary", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(had_reset(&args));

        let args = ["--untagged", "--tag", "--binary"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));

        let args = ["--untagged", "--tag", "-b"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));

        let args = ["--binary", "--tag"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));

        let args = ["--tag", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));

        let args = ["--text", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));

        let args = ["--binary", "--untagged"]
            .iter()
            .map(|&s| s.into())
            .collect::<Vec<OsString>>();
        assert!(!had_reset(&args));
    }

    #[test]
    fn test_prompt_asterisk() {
        assert!(!prompt_asterisk(true, false, false));
        assert!(!prompt_asterisk(false, false, true));
        assert!(prompt_asterisk(false, true, false));
        assert!(!prompt_asterisk(false, false, false));
    }

    #[test]
    fn test_calculate_length() {
        assert_eq!(calculate_blake2b_length(256).unwrap(), Some(32));
        assert_eq!(calculate_blake2b_length(512).unwrap(), None);
        assert_eq!(calculate_blake2b_length(256).unwrap(), Some(32));
        calculate_blake2b_length(255).unwrap_err();

        calculate_blake2b_length(33).unwrap_err();

        calculate_blake2b_length(513).unwrap_err();
    }
}
