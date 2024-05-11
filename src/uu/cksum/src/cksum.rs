// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname, algo
use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use regex::Regex;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::io::{self, stdin, stdout, BufReader, Read, Write};
use std::iter;
use std::path::Path;
use uucore::checksum::cksum_output;
use uucore::error::set_exit_code;
use uucore::{
    encoding,
    error::{FromIo, UError, UResult, USimpleError},
    format_usage, help_about, help_section, help_usage, show,
    sum::{
        div_ceil, Blake2b, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha512, Sm3,
        BSD, CRC, SYSV,
    },
};

const USAGE: &str = help_usage!("cksum.md");
const ABOUT: &str = help_about!("cksum.md");
const AFTER_HELP: &str = help_section!("after help", "cksum.md");

const ALGORITHM_OPTIONS_SYSV: &str = "sysv";
const ALGORITHM_OPTIONS_BSD: &str = "bsd";
const ALGORITHM_OPTIONS_CRC: &str = "crc";
const ALGORITHM_OPTIONS_MD5: &str = "md5";
const ALGORITHM_OPTIONS_SHA1: &str = "sha1";
const ALGORITHM_OPTIONS_SHA224: &str = "sha224";
const ALGORITHM_OPTIONS_SHA256: &str = "sha256";
const ALGORITHM_OPTIONS_SHA384: &str = "sha384";
const ALGORITHM_OPTIONS_SHA512: &str = "sha512";
const ALGORITHM_OPTIONS_BLAKE2B: &str = "blake2b";
const ALGORITHM_OPTIONS_SM3: &str = "sm3";

#[derive(Debug)]
enum CkSumError {
    RawMultipleFiles,
}

#[derive(Debug, PartialEq)]
enum OutputFormat {
    Hexadecimal,
    Raw,
    Base64,
}

impl UError for CkSumError {
    fn code(&self) -> i32 {
        match self {
            Self::RawMultipleFiles => 1,
        }
    }
}

impl Error for CkSumError {}

impl Display for CkSumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawMultipleFiles => {
                write!(f, "the --raw option is not supported with multiple files")
            }
        }
    }
}

fn detect_algo(
    algo: &str,
    length: Option<usize>,
) -> (&'static str, Box<dyn Digest + 'static>, usize) {
    match algo {
        ALGORITHM_OPTIONS_SYSV => (
            ALGORITHM_OPTIONS_SYSV,
            Box::new(SYSV::new()) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_BSD => (
            ALGORITHM_OPTIONS_BSD,
            Box::new(BSD::new()) as Box<dyn Digest>,
            1024,
        ),
        ALGORITHM_OPTIONS_CRC => (
            ALGORITHM_OPTIONS_CRC,
            Box::new(CRC::new()) as Box<dyn Digest>,
            256,
        ),
        ALGORITHM_OPTIONS_MD5 => (
            ALGORITHM_OPTIONS_MD5,
            Box::new(Md5::new()) as Box<dyn Digest>,
            128,
        ),
        ALGORITHM_OPTIONS_SHA1 => (
            ALGORITHM_OPTIONS_SHA1,
            Box::new(Sha1::new()) as Box<dyn Digest>,
            160,
        ),
        ALGORITHM_OPTIONS_SHA224 => (
            ALGORITHM_OPTIONS_SHA224,
            Box::new(Sha224::new()) as Box<dyn Digest>,
            224,
        ),
        ALGORITHM_OPTIONS_SHA256 => (
            ALGORITHM_OPTIONS_SHA256,
            Box::new(Sha256::new()) as Box<dyn Digest>,
            256,
        ),
        ALGORITHM_OPTIONS_SHA384 => (
            ALGORITHM_OPTIONS_SHA384,
            Box::new(Sha384::new()) as Box<dyn Digest>,
            384,
        ),
        ALGORITHM_OPTIONS_SHA512 => (
            ALGORITHM_OPTIONS_SHA512,
            Box::new(Sha512::new()) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_BLAKE2B => (
            ALGORITHM_OPTIONS_BLAKE2B,
            Box::new(if let Some(length) = length {
                Blake2b::with_output_bytes(length)
            } else {
                Blake2b::new()
            }) as Box<dyn Digest>,
            512,
        ),
        ALGORITHM_OPTIONS_SM3 => (
            ALGORITHM_OPTIONS_SM3,
            Box::new(Sm3::new()) as Box<dyn Digest>,
            512,
        ),
        _ => unreachable!("unknown algorithm: clap should have prevented this case"),
    }
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
        return Err(Box::new(CkSumError::RawMultipleFiles));
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

        let (sum_hex, sz) = digest_read(&mut options.digest, &mut file, options.output_bits)
            .map_err_context(|| "failed to read input".to_string())?;
        if filename.is_dir() {
            show!(USimpleError::new(
                1,
                format!("{}: Is a directory", filename.display())
            ));
            continue;
        }
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
        match (options.algo_name, not_file) {
            (ALGORITHM_OPTIONS_SYSV, true) => println!(
                "{} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (ALGORITHM_OPTIONS_SYSV, false) => println!(
                "{} {} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            (ALGORITHM_OPTIONS_BSD, true) => println!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits)
            ),
            (ALGORITHM_OPTIONS_BSD, false) => println!(
                "{:0bsd_width$} {:bsd_width$} {}",
                sum.parse::<u16>().unwrap(),
                div_ceil(sz, options.output_bits),
                filename.display()
            ),
            (ALGORITHM_OPTIONS_CRC, true) => println!("{sum} {sz}"),
            (ALGORITHM_OPTIONS_CRC, false) => println!("{sum} {sz} {}", filename.display()),
            (ALGORITHM_OPTIONS_BLAKE2B, _) if options.tag => {
                if let Some(length) = options.length {
                    // Multiply by 8 here, as we want to print the length in bits.
                    println!("BLAKE2b-{} ({}) = {sum}", length * 8, filename.display());
                } else {
                    println!("BLAKE2b ({}) = {sum}", filename.display());
                }
            }
            _ => {
                if options.tag {
                    println!(
                        "{} ({}) = {sum}",
                        options.algo_name.to_ascii_uppercase(),
                        filename.display()
                    );
                } else {
                    let prefix = if options.asterisk { "*" } else { " " };
                    println!("{sum} {prefix}{}", filename.display());
                }
            }
        }
    }

    Ok(())
}

fn digest_read<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    output_bits: usize,
) -> io::Result<(String, usize)> {
    digest.reset();

    // Read bytes from `reader` and write those bytes to `digest`.
    //
    // If `binary` is `false` and the operating system is Windows, then
    // `DigestWriter` replaces "\r\n" with "\n" before it writes the
    // bytes into `digest`. Otherwise, it just inserts the bytes as-is.
    //
    // In order to support replacing "\r\n", we must call `finalize()`
    // in order to support the possibility that the last character read
    // from the reader was "\r". (This character gets buffered by
    // `DigestWriter` and only written if the following character is
    // "\n". But when "\r" is the last character read, we need to force
    // it to be written.)
    let mut digest_writer = DigestWriter::new(digest, true);
    let output_size = std::io::copy(reader, &mut digest_writer)? as usize;
    digest_writer.finalize();

    if digest.output_bits() > 0 {
        Ok((digest.result_str(), output_size))
    } else {
        // Assume it's SHAKE.  result_str() doesn't work with shake (as of 8/30/2016)
        let mut bytes = vec![0; (output_bits + 7) / 8];
        digest.hash_finalize(&mut bytes);
        Ok((hex::encode(bytes), output_size))
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
fn had_reset(args: &[String]) -> bool {
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

/// Calculates the length of the digest for the given algorithm.
fn calculate_length(algo_name: &str, length: usize) -> UResult<Option<usize>> {
    match length {
        0 => Ok(None),
        n if n % 8 != 0 => {
            uucore::show_error!("invalid length: \u{2018}{length}\u{2019}");
            Err(io::Error::new(io::ErrorKind::InvalidInput, "length is not a multiple of 8").into())
        }
        n if n > 512 => {
            uucore::show_error!("invalid length: \u{2018}{length}\u{2019}");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "maximum digest length for \u{2018}BLAKE2b\u{2019} is 512 bits",
            )
            .into())
        }
        n => {
            if algo_name == ALGORITHM_OPTIONS_BLAKE2B {
                // Divide by 8, as our blake2b implementation expects bytes instead of bits.
                Ok(Some(n / 8))
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--length is only supported with --algorithm=blake2b",
                )
                .into())
            }
        }
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

    let args: Vec<String> = std::env::args().collect();
    let had_reset = had_reset(&args);

    let asterisk: bool = prompt_asterisk(tag, binary_flag, had_reset);

    Ok((tag, asterisk))
}

/***
 * Do the checksum validation (can be strict or not)
*/
fn perform_checksum_validation<'a, I>(
    files: I,
    strict: bool,
    algo_name_input: Option<&str>,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // Regexp to handle the two input formats:
    // 1. <algo>[-<bits>] (<filename>) = <checksum>
    // 2. <checksum> [* ]<filename>
    let regex_pattern = r"^(?P<algo>\w+)(-(?P<bits>\d+))?\s?\((?P<filename1>.*)\) = (?P<checksum1>[a-fA-F0-9]+)$|^(?P<checksum2>[a-fA-F0-9]+)\s[* ](?P<filename2>.*)";
    let re = Regex::new(regex_pattern).unwrap();

    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        let mut bad_format = 0;
        let mut failed_cksum = 0;
        let mut failed_open_file = 0;
        let mut properly_formatted = false;
        let input_is_stdin = filename_input == OsStr::new("-");

        let file: Box<dyn Read> = if input_is_stdin {
            Box::new(stdin()) // Use stdin if "-" is specified
        } else {
            match File::open(filename_input) {
                Ok(f) => Box::new(f),
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "{}: No such file or directory",
                            filename_input.to_string_lossy()
                        ),
                    )
                    .into());
                }
            }
        };
        let reader = BufReader::new(file);

        // for each line in the input, check if it is a valid checksum line
        for line in reader.lines() {
            let line = line.unwrap_or_else(|_| String::new());
            if let Some(caps) = re.captures(&line) {
                properly_formatted = true;

                // Determine what kind of file input we had
                // we need it for case "--check -a sm3 <file>" when <file> is
                // <algo>[-<bits>] (<filename>) = <checksum>
                let algo_based_format =
                    caps.name("filename1").is_some() && caps.name("checksum1").is_some();

                let filename_to_check = caps
                    .name("filename1")
                    .or(caps.name("filename2"))
                    .unwrap()
                    .as_str();
                let expected_checksum = caps
                    .name("checksum1")
                    .or(caps.name("checksum2"))
                    .unwrap()
                    .as_str();

                // If the algo_name is provided, we use it, otherwise we try to detect it
                let algo_details = if algo_based_format {
                    // When the algo-based format is matched, extract details from regex captures
                    let algorithm = caps.name("algo").map_or("", |m| m.as_str()).to_lowercase();
                    let bits = caps
                        .name("bits")
                        .map(|m| m.as_str().parse::<usize>().unwrap() / 8);
                    (algorithm, bits)
                } else if let Some(a) = algo_name_input {
                    // When a specific algorithm name is input, use it and default bits to None
                    (a.to_lowercase(), None)
                } else {
                    // Default case if no algorithm is specified and non-algo based format is matched
                    (String::new(), None)
                };
                if algo_based_format
                    && algo_name_input.map_or(false, |input| algo_details.0 != input)
                {
                    bad_format += 1;
                    continue;
                }
                if algo_details.0.is_empty() {
                    // we haven't been able to detect the algo name. No point to continue
                    properly_formatted = false;
                    continue;
                }
                let (_, mut algo, bits) = detect_algo(&algo_details.0, algo_details.1);

                // manage the input file
                let file_to_check: Box<dyn Read> = if filename_to_check == "-" {
                    Box::new(stdin()) // Use stdin if "-" is specified in the checksum file
                } else {
                    match File::open(filename_to_check) {
                        Ok(f) => Box::new(f),
                        Err(err) => {
                            show!(err.map_err_context(|| format!(
                                "Failed to open file: {}",
                                filename_to_check
                            )));
                            failed_open_file += 1;
                            // we could not open the file but we want to continue
                            continue;
                        }
                    }
                };
                let mut file_reader = BufReader::new(file_to_check);
                // Read the file and calculate the checksum
                let (calculated_checksum, _) =
                    digest_read(&mut algo, &mut file_reader, bits).unwrap();

                // Do the checksum validation
                if expected_checksum == calculated_checksum {
                    println!("{}: OK", filename_to_check);
                } else {
                    println!("{}: FAILED", filename_to_check);
                    failed_cksum += 1;
                }
            } else {
                bad_format += 1;
            }
        }

        // not a single line correctly formatted found
        // return an error
        if !properly_formatted {
            uucore::show_error!(
                "{}: no properly formatted checksum lines found",
                filename_input.to_string_lossy()
            );
            set_exit_code(1);
        }
        // strict means that we should have an exit code.
        if strict && bad_format > 0 {
            set_exit_code(1);
        }

        // if we have any failed checksum verification, we set an exit code
        if failed_cksum > 0 || failed_open_file > 0 {
            set_exit_code(1);
        }

        // if any incorrectly formatted line, show it
        cksum_output(bad_format, failed_cksum, failed_open_file);
    }
    Ok(())
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
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--check is not supported with --algorithm={bsd,sysv,crc}",
        )
        .into());
    }

    if check {
        let text_flag: bool = matches.get_flag(options::TEXT);
        let binary_flag: bool = matches.get_flag(options::BINARY);
        let strict = matches.get_flag(options::STRICT);

        if (binary_flag || text_flag) && check {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the --binary and --text options are meaningless when verifying checksums",
            )
            .into());
        }
        // Determine the appropriate algorithm option to pass
        let algo_option = if algo_name.is_empty() {
            None
        } else {
            Some(algo_name)
        };

        // Execute the checksum validation based on the presence of files or the use of stdin
        return match matches.get_many::<String>(options::FILE) {
            Some(files) => perform_checksum_validation(files.map(OsStr::new), strict, algo_option),
            None => perform_checksum_validation(iter::once(OsStr::new("-")), strict, algo_option),
        };
    }

    let input_length = matches.get_one::<usize>(options::LENGTH);

    let length = match input_length {
        Some(length) => calculate_length(algo_name, *length)?,
        None => None,
    };

    let (tag, asterisk) = handle_tag_text_binary_flags(&matches)?;

    let (name, algo, bits) = detect_algo(algo_name, length);

    let output_format = if matches.get_flag(options::RAW) {
        OutputFormat::Raw
    } else if matches.get_flag(options::BASE64) {
        OutputFormat::Base64
    } else {
        OutputFormat::Hexadecimal
    };

    let opts = Options {
        algo_name: name,
        digest: algo,
        output_bits: bits,
        length,
        tag,
        output_format,
        asterisk,
    };

    match matches.get_many::<String>(options::FILE) {
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
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ALGORITHM)
                .long(options::ALGORITHM)
                .short('a')
                .help("select the digest type to use. See DIGEST below")
                .value_name("ALGORITHM")
                .value_parser([
                    ALGORITHM_OPTIONS_SYSV,
                    ALGORITHM_OPTIONS_BSD,
                    ALGORITHM_OPTIONS_CRC,
                    ALGORITHM_OPTIONS_MD5,
                    ALGORITHM_OPTIONS_SHA1,
                    ALGORITHM_OPTIONS_SHA224,
                    ALGORITHM_OPTIONS_SHA256,
                    ALGORITHM_OPTIONS_SHA384,
                    ALGORITHM_OPTIONS_SHA512,
                    ALGORITHM_OPTIONS_BLAKE2B,
                    ALGORITHM_OPTIONS_SM3,
                ]),
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
        .after_help(AFTER_HELP)
}

#[cfg(test)]
mod tests {
    use super::had_reset;
    use crate::calculate_length;
    use crate::prompt_asterisk;

    #[test]
    fn test_had_reset() {
        let args = ["--binary", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(had_reset(&args));

        let args = ["-b", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(had_reset(&args));

        let args = ["-b", "--binary", "--tag", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(had_reset(&args));

        let args = ["--untagged", "--tag", "--binary"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(!had_reset(&args));

        let args = ["--untagged", "--tag", "-b"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(!had_reset(&args));

        let args = ["--binary", "--tag"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(!had_reset(&args));

        let args = ["--tag", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(!had_reset(&args));

        let args = ["--text", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        assert!(!had_reset(&args));

        let args = ["--binary", "--untagged"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
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
        assert_eq!(
            calculate_length(crate::ALGORITHM_OPTIONS_BLAKE2B, 256).unwrap(),
            Some(32)
        );

        calculate_length(crate::ALGORITHM_OPTIONS_BLAKE2B, 255).unwrap_err();

        calculate_length(crate::ALGORITHM_OPTIONS_SHA256, 33).unwrap_err();

        calculate_length(crate::ALGORITHM_OPTIONS_BLAKE2B, 513).unwrap_err();

        calculate_length(crate::ALGORITHM_OPTIONS_SHA256, 256).unwrap_err();
    }
}
