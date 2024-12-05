// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore anotherfile invalidchecksum regexes JWZG FFFD xffname prefixfilename

use data_encoding::BASE64;
use os_display::Quotable;
use regex::bytes::{Captures, Regex};
use std::{
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::{self, stdin, BufReader, Read, Write},
    path::Path,
    str,
};

use crate::{
    error::{set_exit_code, FromIo, UError, UResult, USimpleError},
    os_str_as_bytes, os_str_from_bytes, read_os_string_lines, show, show_error, show_warning_caps,
    sum::{
        Blake2b, Blake3, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha3_224,
        Sha3_256, Sha3_384, Sha3_512, Sha512, Shake128, Shake256, Sm3, BSD, CRC, SYSV,
    },
    util_name,
};
use thiserror::Error;

pub const ALGORITHM_OPTIONS_SYSV: &str = "sysv";
pub const ALGORITHM_OPTIONS_BSD: &str = "bsd";
pub const ALGORITHM_OPTIONS_CRC: &str = "crc";
pub const ALGORITHM_OPTIONS_MD5: &str = "md5";
pub const ALGORITHM_OPTIONS_SHA1: &str = "sha1";
pub const ALGORITHM_OPTIONS_SHA3: &str = "sha3";

pub const ALGORITHM_OPTIONS_SHA224: &str = "sha224";
pub const ALGORITHM_OPTIONS_SHA256: &str = "sha256";
pub const ALGORITHM_OPTIONS_SHA384: &str = "sha384";
pub const ALGORITHM_OPTIONS_SHA512: &str = "sha512";
pub const ALGORITHM_OPTIONS_BLAKE2B: &str = "blake2b";
pub const ALGORITHM_OPTIONS_BLAKE3: &str = "blake3";
pub const ALGORITHM_OPTIONS_SM3: &str = "sm3";
pub const ALGORITHM_OPTIONS_SHAKE128: &str = "shake128";
pub const ALGORITHM_OPTIONS_SHAKE256: &str = "shake256";

pub const SUPPORTED_ALGORITHMS: [&str; 15] = [
    ALGORITHM_OPTIONS_SYSV,
    ALGORITHM_OPTIONS_BSD,
    ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_MD5,
    ALGORITHM_OPTIONS_SHA1,
    ALGORITHM_OPTIONS_SHA3,
    ALGORITHM_OPTIONS_SHA224,
    ALGORITHM_OPTIONS_SHA256,
    ALGORITHM_OPTIONS_SHA384,
    ALGORITHM_OPTIONS_SHA512,
    ALGORITHM_OPTIONS_BLAKE2B,
    ALGORITHM_OPTIONS_BLAKE3,
    ALGORITHM_OPTIONS_SM3,
    ALGORITHM_OPTIONS_SHAKE128,
    ALGORITHM_OPTIONS_SHAKE256,
];

pub struct HashAlgorithm {
    pub name: &'static str,
    pub create_fn: Box<dyn Fn() -> Box<dyn Digest + 'static>>,
    pub bits: usize,
}

/// This structure holds the count of checksum test lines' outcomes.
#[derive(Default)]
struct ChecksumResult {
    /// Number of lines in the file where the computed checksum MATCHES
    /// the expectation.
    pub correct: u32,
    /// Number of lines in the file where the computed checksum DIFFERS
    /// from the expectation.
    pub failed_cksum: u32,
    pub failed_open_file: u32,
    /// Number of improperly formatted lines.
    pub bad_format: u32,
    /// Total number of non-empty, non-comment lines.
    pub total: u32,
}

impl ChecksumResult {
    #[inline]
    fn total_properly_formatted(&self) -> u32 {
        self.total - self.bad_format
    }
}

/// Represents a reason for which the processing of a checksum line
/// could not proceed to digest comparison.
enum LineCheckError {
    /// a generic UError was encountered in sub-functions
    UError(Box<dyn UError>),
    /// the computed checksum digest differs from the expected one
    DigestMismatch,
    /// the line is empty or is a comment
    Skipped,
    /// the line has a formatting error
    ImproperlyFormatted,
    /// file exists but is impossible to read
    CantOpenFile,
    /// there is nothing at the given path
    FileNotFound,
    /// the given path leads to a directory
    FileIsDirectory,
}

impl From<Box<dyn UError>> for LineCheckError {
    fn from(value: Box<dyn UError>) -> Self {
        Self::UError(value)
    }
}

impl From<ChecksumError> for LineCheckError {
    fn from(value: ChecksumError) -> Self {
        Self::UError(Box::new(value))
    }
}

/// Represents an error that was encountered when processing a checksum file.
enum FileCheckError {
    /// a generic UError was encountered in sub-functions
    UError(Box<dyn UError>),
    /// the checksum file is improperly formatted.
    ImproperlyFormatted,
    /// reading of the checksum file failed
    CantOpenChecksumFile,
}

impl From<Box<dyn UError>> for FileCheckError {
    fn from(value: Box<dyn UError>) -> Self {
        Self::UError(value)
    }
}

impl From<ChecksumError> for FileCheckError {
    fn from(value: ChecksumError) -> Self {
        Self::UError(Box::new(value))
    }
}

/// This struct regroups CLI flags.
#[derive(Debug, Default, Clone, Copy)]
pub struct ChecksumOptions {
    pub binary: bool,
    pub ignore_missing: bool,
    pub quiet: bool,
    pub status: bool,
    pub strict: bool,
    pub warn: bool,
}

#[derive(Debug, Error)]
pub enum ChecksumError {
    #[error("the --raw option is not supported with multiple files")]
    RawMultipleFiles,
    #[error("the --ignore-missing option is meaningful only when verifying checksums")]
    IgnoreNotCheck,
    #[error("the --strict option is meaningful only when verifying checksums")]
    StrictNotCheck,
    #[error("the --quiet option is meaningful only when verifying checksums")]
    QuietNotCheck,
    #[error("Invalid output size for SHA3 (expected 224, 256, 384, or 512)")]
    InvalidOutputSizeForSha3,
    #[error("--bits required for SHA3")]
    BitsRequiredForSha3,
    #[error("--bits required for SHAKE128")]
    BitsRequiredForShake128,
    #[error("--bits required for SHAKE256")]
    BitsRequiredForShake256,
    #[error("unknown algorithm: clap should have prevented this case")]
    UnknownAlgorithm,
    #[error("length is not a multiple of 8")]
    InvalidLength,
    #[error("--length is only supported with --algorithm=blake2b")]
    LengthOnlyForBlake2b,
    #[error("the --binary and --text options are meaningless when verifying checksums")]
    BinaryTextConflict,
    #[error("--check is not supported with --algorithm={{bsd,sysv,crc}}")]
    AlgorithmNotSupportedWithCheck,
    #[error("You cannot combine multiple hash algorithms!")]
    CombineMultipleAlgorithms,
    #[error("Needs an algorithm to hash with.\nUse --help for more information.")]
    NeedAlgorithmToHash,
}

impl UError for ChecksumError {
    fn code(&self) -> i32 {
        1
    }
}

/// Creates a SHA3 hasher instance based on the specified bits argument.
///
/// # Returns
///
/// Returns a UResult of a tuple containing the algorithm name, the hasher instance, and
/// the output length in bits or an Err if an unsupported output size is provided, or if
/// the `--bits` flag is missing.
pub fn create_sha3(bits: Option<usize>) -> UResult<HashAlgorithm> {
    match bits {
        Some(224) => Ok(HashAlgorithm {
            name: "SHA3_224",
            create_fn: Box::new(|| Box::new(Sha3_224::new())),
            bits: 224,
        }),
        Some(256) => Ok(HashAlgorithm {
            name: "SHA3_256",
            create_fn: Box::new(|| Box::new(Sha3_256::new())),
            bits: 256,
        }),
        Some(384) => Ok(HashAlgorithm {
            name: "SHA3_384",
            create_fn: Box::new(|| Box::new(Sha3_384::new())),
            bits: 384,
        }),
        Some(512) => Ok(HashAlgorithm {
            name: "SHA3_512",
            create_fn: Box::new(|| Box::new(Sha3_512::new())),
            bits: 512,
        }),

        Some(_) => Err(ChecksumError::InvalidOutputSizeForSha3.into()),
        None => Err(ChecksumError::BitsRequiredForSha3.into()),
    }
}

#[allow(clippy::comparison_chain)]
fn cksum_output(res: &ChecksumResult, status: bool) {
    if res.bad_format == 1 {
        show_warning_caps!("{} line is improperly formatted", res.bad_format);
    } else if res.bad_format > 1 {
        show_warning_caps!("{} lines are improperly formatted", res.bad_format);
    }

    if !status {
        if res.failed_cksum == 1 {
            show_warning_caps!("{} computed checksum did NOT match", res.failed_cksum);
        } else if res.failed_cksum > 1 {
            show_warning_caps!("{} computed checksums did NOT match", res.failed_cksum);
        }
    }
    if res.failed_open_file == 1 {
        show_warning_caps!("{} listed file could not be read", res.failed_open_file);
    } else if res.failed_open_file > 1 {
        show_warning_caps!("{} listed files could not be read", res.failed_open_file);
    }
}

/// Print a "no properly formatted lines" message in stderr
#[inline]
fn log_no_properly_formatted(filename: String) {
    show_error!("{filename}: no properly formatted checksum lines found");
}

/// Represents the different outcomes that can happen to a file
/// that is being checked.
#[derive(Debug, Clone, Copy)]
enum FileChecksumResult {
    Ok,
    Failed,
    CantOpen,
}

impl FileChecksumResult {
    /// Creates a `FileChecksumResult` from a digest comparison that
    /// either succeeded or failed.
    fn from_bool(checksum_correct: bool) -> Self {
        if checksum_correct {
            FileChecksumResult::Ok
        } else {
            FileChecksumResult::Failed
        }
    }

    /// The cli options might prevent to display on the outcome of the
    /// comparison on STDOUT.
    fn can_display(&self, opts: ChecksumOptions) -> bool {
        match self {
            FileChecksumResult::Ok => !opts.status && !opts.quiet,
            FileChecksumResult::Failed => !opts.status,
            FileChecksumResult::CantOpen => true,
        }
    }
}

impl Display for FileChecksumResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileChecksumResult::Ok => write!(f, "OK"),
            FileChecksumResult::Failed => write!(f, "FAILED"),
            FileChecksumResult::CantOpen => write!(f, "FAILED open or read"),
        }
    }
}

/// Print to the given buffer the checksum validation status of a file which
/// name might contain non-utf-8 characters.
fn print_file_report<W: Write>(
    mut w: W,
    filename: &[u8],
    result: FileChecksumResult,
    prefix: &str,
    opts: ChecksumOptions,
) {
    if result.can_display(opts) {
        let _ = write!(w, "{prefix}");
        let _ = w.write_all(filename);
        let _ = writeln!(w, ": {result}");
    }
}

pub fn detect_algo(algo: &str, length: Option<usize>) -> UResult<HashAlgorithm> {
    match algo {
        ALGORITHM_OPTIONS_SYSV => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SYSV,
            create_fn: Box::new(|| Box::new(SYSV::new())),
            bits: 512,
        }),
        ALGORITHM_OPTIONS_BSD => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_BSD,
            create_fn: Box::new(|| Box::new(BSD::new())),
            bits: 1024,
        }),
        ALGORITHM_OPTIONS_CRC => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_CRC,
            create_fn: Box::new(|| Box::new(CRC::new())),
            bits: 256,
        }),
        ALGORITHM_OPTIONS_MD5 | "md5sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_MD5,
            create_fn: Box::new(|| Box::new(Md5::new())),
            bits: 128,
        }),
        ALGORITHM_OPTIONS_SHA1 | "sha1sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SHA1,
            create_fn: Box::new(|| Box::new(Sha1::new())),
            bits: 160,
        }),
        ALGORITHM_OPTIONS_SHA224 | "sha224sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SHA224,
            create_fn: Box::new(|| Box::new(Sha224::new())),
            bits: 224,
        }),
        ALGORITHM_OPTIONS_SHA256 | "sha256sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SHA256,
            create_fn: Box::new(|| Box::new(Sha256::new())),
            bits: 256,
        }),
        ALGORITHM_OPTIONS_SHA384 | "sha384sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SHA384,
            create_fn: Box::new(|| Box::new(Sha384::new())),
            bits: 384,
        }),
        ALGORITHM_OPTIONS_SHA512 | "sha512sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SHA512,
            create_fn: Box::new(|| Box::new(Sha512::new())),
            bits: 512,
        }),
        ALGORITHM_OPTIONS_BLAKE2B | "b2sum" => {
            // Set default length to 512 if None
            let bits = length.unwrap_or(512);
            if bits == 512 {
                Ok(HashAlgorithm {
                    name: ALGORITHM_OPTIONS_BLAKE2B,
                    create_fn: Box::new(move || Box::new(Blake2b::new())),
                    bits: 512,
                })
            } else {
                Ok(HashAlgorithm {
                    name: ALGORITHM_OPTIONS_BLAKE2B,
                    create_fn: Box::new(move || Box::new(Blake2b::with_output_bytes(bits))),
                    bits,
                })
            }
        }
        ALGORITHM_OPTIONS_BLAKE3 | "b3sum" => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_BLAKE3,
            create_fn: Box::new(|| Box::new(Blake3::new())),
            bits: 256,
        }),
        ALGORITHM_OPTIONS_SM3 => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SM3,
            create_fn: Box::new(|| Box::new(Sm3::new())),
            bits: 512,
        }),
        ALGORITHM_OPTIONS_SHAKE128 | "shake128sum" => {
            let bits =
                length.ok_or_else(|| USimpleError::new(1, "--bits required for SHAKE128"))?;
            Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHAKE128,
                create_fn: Box::new(|| Box::new(Shake128::new())),
                bits,
            })
        }
        ALGORITHM_OPTIONS_SHAKE256 | "shake256sum" => {
            let bits =
                length.ok_or_else(|| USimpleError::new(1, "--bits required for SHAKE256"))?;
            Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHAKE256,
                create_fn: Box::new(|| Box::new(Shake256::new())),
                bits,
            })
        }
        //ALGORITHM_OPTIONS_SHA3 | "sha3" => (
        _ if algo.starts_with("sha3") => create_sha3(length),

        _ => Err(ChecksumError::UnknownAlgorithm.into()),
    }
}

// Regexp to handle the three input formats:
// 1. <algo>[-<bits>] (<filename>) = <checksum>
//    algo must be uppercase or b (for blake2b)
// 2. <checksum> [* ]<filename>
// 3. <checksum> [*]<filename> (only one space)
const ALGO_BASED_REGEX: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>(?-u:.*))\)\s*=\s*(?P<checksum>[a-fA-F0-9]+)$";
const ALGO_BASED_REGEX_BASE64: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>(?-u:.*))\)\s*=\s*(?P<checksum>[A-Za-z0-9+/]+={0,2})$";

const DOUBLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s{2}(?P<filename>(?-u:.*))$";

// In this case, we ignore the *
const SINGLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s(?P<filename>\*?(?-u:.*))$";

fn get_filename_for_output(filename: &OsStr, input_is_stdin: bool) -> String {
    if input_is_stdin {
        "standard input"
    } else {
        filename.to_str().unwrap()
    }
    .maybe_quote()
    .to_string()
}

/// Determines the appropriate regular expression to use based on the provided lines.
fn determine_regex(line: impl AsRef<OsStr>) -> Option<(Regex, bool)> {
    let regexes = [
        (Regex::new(ALGO_BASED_REGEX).unwrap(), true),
        (Regex::new(DOUBLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(SINGLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(ALGO_BASED_REGEX_BASE64).unwrap(), true),
    ];

    let line_bytes = os_str_as_bytes(line.as_ref()).expect("UTF-8 decoding failed");
    for (regex, is_algo_based) in &regexes {
        if regex.is_match(line_bytes) {
            return Some((regex.clone(), *is_algo_based));
        }
    }

    None
}

/// Extract the expected digest from the checksum string
fn get_expected_digest_as_hex_string(caps: &Captures, chosen_regex: &Regex) -> Option<String> {
    // Unwraps are safe, ensured by regex.
    let ck = caps.name("checksum").unwrap().as_bytes();

    if chosen_regex.as_str() == ALGO_BASED_REGEX_BASE64 {
        BASE64.decode(ck).map(hex::encode).ok()
    } else if ck.len() % 2 == 0 {
        Some(str::from_utf8(ck).unwrap().to_string())
    } else {
        // If the length of the digest is not a multiple of 2, then it
        // must be improperly formatted (1 hex digit is 2 characters)
        None
    }
}

/// Returns a reader that reads from the specified file, or from stdin if `filename_to_check` is "-".
fn get_file_to_check(
    filename: &OsStr,
    opts: ChecksumOptions,
) -> Result<Box<dyn Read>, LineCheckError> {
    let filename_bytes = os_str_as_bytes(filename).expect("UTF-8 error");
    let filename_lossy = String::from_utf8_lossy(filename_bytes);
    if filename == "-" {
        Ok(Box::new(stdin())) // Use stdin if "-" is specified in the checksum file
    } else {
        let failed_open = || {
            print_file_report(
                std::io::stdout(),
                filename_bytes,
                FileChecksumResult::CantOpen,
                "",
                opts,
            );
        };
        match File::open(filename) {
            Ok(f) => {
                if f.metadata()
                    .map_err(|_| LineCheckError::CantOpenFile)?
                    .is_dir()
                {
                    show!(USimpleError::new(
                        1,
                        format!("{filename_lossy}: Is a directory")
                    ));
                    // also regarded as a failed open
                    failed_open();
                    Err(LineCheckError::FileIsDirectory)
                } else {
                    Ok(Box::new(f))
                }
            }
            Err(err) => {
                if !opts.ignore_missing {
                    // yes, we have both stderr and stdout here
                    show!(err.map_err_context(|| filename_lossy.to_string()));
                    failed_open();
                }
                // we could not open the file but we want to continue
                Err(LineCheckError::FileNotFound)
            }
        }
    }
}

/// Returns a reader to the list of checksums
fn get_input_file(filename: &OsStr) -> UResult<Box<dyn Read>> {
    match File::open(filename) {
        Ok(f) => {
            if f.metadata()?.is_dir() {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{}: Is a directory", filename.to_string_lossy()),
                )
                .into())
            } else {
                Ok(Box::new(f))
            }
        }
        Err(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{}: No such file or directory", filename.to_string_lossy()),
        )
        .into()),
    }
}

/// Extracts the algorithm name and length from the regex captures if the algo-based format is matched.
fn identify_algo_name_and_length(
    caps: &Captures,
    algo_name_input: Option<&str>,
) -> Option<(String, Option<usize>)> {
    // When the algo-based format is matched, extract details from regex captures
    let algorithm = caps
        .name("algo")
        .map_or(String::new(), |m| {
            String::from_utf8(m.as_bytes().into()).unwrap()
        })
        .to_lowercase();

    // check if we are called with XXXsum (example: md5sum) but we detected a different algo parsing the file
    // (for example SHA1 (f) = d...)
    // Also handle the case cksum -s sm3 but the file contains other formats
    if algo_name_input.is_some() && algo_name_input != Some(&algorithm) {
        return None;
    }

    if !SUPPORTED_ALGORITHMS.contains(&algorithm.as_str()) {
        // Not supported algo, leave early
        return None;
    }

    let bits = caps.name("bits").map_or(Some(None), |m| {
        let bits_value = String::from_utf8(m.as_bytes().into())
            .unwrap()
            .parse::<usize>()
            .unwrap();
        if bits_value % 8 == 0 {
            Some(Some(bits_value / 8))
        } else {
            None // Return None to signal a divisibility issue
        }
    })?;

    Some((algorithm, bits))
}

/// Parses a checksum line, detect the algorithm to use, read the file and produce
/// its digest, and compare it to the expected value.
///
/// Returns `Ok(bool)` if the comparison happened, bool indicates if the digest
/// matched the expected.
/// If the comparison didn't happen, return a `LineChecksumError`.
#[allow(clippy::too_many_arguments)]
fn process_checksum_line(
    filename_input: &OsStr,
    line: &OsStr,
    i: usize,
    cli_algo_name: Option<&str>,
    cli_algo_length: Option<usize>,
    opts: ChecksumOptions,
) -> Result<(), LineCheckError> {
    let line_bytes = os_str_as_bytes(line)?;

    // early return on empty or commented lines.
    if line.is_empty() || line_bytes.starts_with(b"#") {
        return Err(LineCheckError::Skipped);
    }

    let (chosen_regex, is_algo_based_format) =
        determine_regex(line).ok_or(LineCheckError::ImproperlyFormatted)?;

    if let Some(caps) = chosen_regex.captures(line_bytes) {
        let mut filename_to_check = caps.name("filename").unwrap().as_bytes();

        if filename_to_check.starts_with(b"*")
            && i == 0
            && chosen_regex.as_str() == SINGLE_SPACE_REGEX
        {
            // Remove the leading asterisk if present - only for the first line
            filename_to_check = &filename_to_check[1..];
        }

        let expected_checksum = get_expected_digest_as_hex_string(&caps, &chosen_regex)
            .ok_or(LineCheckError::ImproperlyFormatted)?;

        // If the algo_name is provided, we use it, otherwise we try to detect it
        let (algo_name, length) = if is_algo_based_format {
            identify_algo_name_and_length(&caps, cli_algo_name)
                .ok_or(LineCheckError::ImproperlyFormatted)?
        } else if let Some(a) = cli_algo_name {
            // When a specific algorithm name is input, use it and use the provided bits
            // except when dealing with blake2b, where we will detect the length
            if cli_algo_name == Some(ALGORITHM_OPTIONS_BLAKE2B) {
                // division by 2 converts the length of the Blake2b checksum from hexadecimal
                // characters to bytes, as each byte is represented by two hexadecimal characters.
                let length = Some(expected_checksum.len() / 2);
                (ALGORITHM_OPTIONS_BLAKE2B.to_string(), length)
            } else {
                (a.to_lowercase(), cli_algo_length)
            }
        } else {
            // Default case if no algorithm is specified and non-algo based format is matched
            return Err(LineCheckError::ImproperlyFormatted);
        };

        let mut algo = detect_algo(&algo_name, length)?;

        let (filename_to_check_unescaped, prefix) = unescape_filename(filename_to_check);

        let real_filename_to_check = os_str_from_bytes(&filename_to_check_unescaped)?;

        // manage the input file
        let file_to_check = get_file_to_check(&real_filename_to_check, opts)?;
        let mut file_reader = BufReader::new(file_to_check);

        // Read the file and calculate the checksum
        let create_fn = &mut algo.create_fn;
        let mut digest = create_fn();
        let (calculated_checksum, _) =
            digest_reader(&mut digest, &mut file_reader, opts.binary, algo.bits).unwrap();

        // Do the checksum validation
        let checksum_correct = expected_checksum == calculated_checksum;
        print_file_report(
            std::io::stdout(),
            filename_to_check,
            FileChecksumResult::from_bool(checksum_correct),
            prefix,
            opts,
        );

        if checksum_correct {
            Ok(())
        } else {
            Err(LineCheckError::DigestMismatch)
        }
    } else {
        if opts.warn {
            let algo = if let Some(algo_name_input) = cli_algo_name {
                algo_name_input.to_uppercase()
            } else {
                "Unknown algorithm".to_string()
            };
            eprintln!(
                "{}: {}: {}: improperly formatted {} checksum line",
                util_name(),
                &filename_input.maybe_quote(),
                i + 1,
                algo
            );
        }

        Err(LineCheckError::ImproperlyFormatted)
    }
}

fn process_checksum_file(
    filename_input: &OsStr,
    cli_algo_name: Option<&str>,
    cli_algo_length: Option<usize>,
    opts: ChecksumOptions,
) -> Result<(), FileCheckError> {
    let mut res = ChecksumResult::default();

    let input_is_stdin = filename_input == OsStr::new("-");

    let file: Box<dyn Read> = if input_is_stdin {
        // Use stdin if "-" is specified
        Box::new(stdin())
    } else {
        match get_input_file(filename_input) {
            Ok(f) => f,
            Err(e) => {
                // Could not read the file, show the error and continue to the next file
                show_error!("{e}");
                set_exit_code(1);
                return Err(FileCheckError::CantOpenChecksumFile);
            }
        }
    };

    let reader = BufReader::new(file);
    let lines = read_os_string_lines(reader).collect::<Vec<_>>();

    for (i, line) in lines.iter().enumerate() {
        let line_result = process_checksum_line(
            filename_input,
            line,
            i,
            cli_algo_name,
            cli_algo_length,
            opts,
        );

        // Match a first time to elude critical UErrors, and increment the total
        // in all cases except on skipped.
        use LineCheckError::*;
        match line_result {
            Err(UError(e)) => return Err(e.into()),
            Err(Skipped) => (),
            _ => res.total += 1,
        }

        // Match a second time to update the right field of `res`.
        match line_result {
            Ok(()) => res.correct += 1,
            Err(DigestMismatch) => res.failed_cksum += 1,
            Err(ImproperlyFormatted) => res.bad_format += 1,
            Err(CantOpenFile | FileIsDirectory) => res.failed_open_file += 1,
            Err(FileNotFound) if !opts.ignore_missing => res.failed_open_file += 1,
            _ => continue,
        };
    }

    // not a single line correctly formatted found
    // return an error
    if res.total_properly_formatted() == 0 {
        if !opts.status {
            log_no_properly_formatted(get_filename_for_output(filename_input, input_is_stdin));
        }
        set_exit_code(1);
        return Err(FileCheckError::ImproperlyFormatted);
    }

    // if any incorrectly formatted line, show it
    cksum_output(&res, opts.status);

    if opts.ignore_missing && res.correct == 0 {
        // we have only bad format
        // and we had ignore-missing
        eprintln!(
            "{}: {}: no file was verified",
            util_name(),
            filename_input.maybe_quote(),
        );
        set_exit_code(1);
    }

    // strict means that we should have an exit code.
    if opts.strict && res.bad_format > 0 {
        set_exit_code(1);
    }

    // if we have any failed checksum verification, we set an exit code
    // except if we have ignore_missing
    if (res.failed_cksum > 0 || res.failed_open_file > 0) && !opts.ignore_missing {
        set_exit_code(1);
    }

    Ok(())
}

/***
 * Do the checksum validation (can be strict or not)
*/
pub fn perform_checksum_validation<'a, I>(
    files: I,
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
    opts: ChecksumOptions,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        use FileCheckError::*;
        match process_checksum_file(filename_input, algo_name_input, length_input, opts) {
            Err(UError(e)) => return Err(e),
            Err(CantOpenChecksumFile | ImproperlyFormatted) | Ok(_) => continue,
        }
    }

    Ok(())
}

pub fn digest_reader<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut BufReader<T>,
    binary: bool,
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
    let mut digest_writer = DigestWriter::new(digest, binary);
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

/// Calculates the length of the digest.
pub fn calculate_blake2b_length(length: usize) -> UResult<Option<usize>> {
    match length {
        0 => Ok(None),
        n if n % 8 != 0 => {
            show_error!("invalid length: \u{2018}{length}\u{2019}");
            Err(io::Error::new(io::ErrorKind::InvalidInput, "length is not a multiple of 8").into())
        }
        n if n > 512 => {
            show_error!("invalid length: \u{2018}{length}\u{2019}");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "maximum digest length for \u{2018}BLAKE2b\u{2019} is 512 bits",
            )
            .into())
        }
        n => {
            // Divide by 8, as our blake2b implementation expects bytes instead of bits.
            if n == 512 {
                // When length is 512, it is blake2b's default.
                // So, don't show it
                Ok(None)
            } else {
                Ok(Some(n / 8))
            }
        }
    }
}

pub fn unescape_filename(filename: &[u8]) -> (Vec<u8>, &'static str) {
    let mut unescaped = Vec::with_capacity(filename.len());
    let mut byte_iter = filename.iter().peekable();
    loop {
        let Some(byte) = byte_iter.next() else {
            break;
        };
        if *byte == b'\\' {
            match byte_iter.next() {
                Some(b'\\') => unescaped.push(b'\\'),
                Some(b'n') => unescaped.push(b'\n'),
                Some(b'r') => unescaped.push(b'\r'),
                Some(x) => {
                    unescaped.push(b'\\');
                    unescaped.push(*x);
                }
                _ => {}
            }
        } else {
            unescaped.push(*byte);
        }
    }
    let prefix = if unescaped == filename { "" } else { "\\" };
    (unescaped, prefix)
}

pub fn escape_filename(filename: &Path) -> (String, &'static str) {
    let original = filename.as_os_str().to_string_lossy();
    let escaped = original
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let prefix = if escaped == original { "" } else { "\\" };
    (escaped, prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_unescape_filename() {
        let (unescaped, prefix) = unescape_filename(b"test\\nfile.txt");
        assert_eq!(unescaped, b"test\nfile.txt");
        assert_eq!(prefix, "\\");
        let (unescaped, prefix) = unescape_filename(b"test\\nfile.txt");
        assert_eq!(unescaped, b"test\nfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename(b"test\\rfile.txt");
        assert_eq!(unescaped, b"test\rfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename(b"test\\\\file.txt");
        assert_eq!(unescaped, b"test\\file.txt");
        assert_eq!(prefix, "\\");
    }

    #[test]
    fn test_escape_filename() {
        let (escaped, prefix) = escape_filename(Path::new("testfile.txt"));
        assert_eq!(escaped, "testfile.txt");
        assert_eq!(prefix, "");

        let (escaped, prefix) = escape_filename(Path::new("test\nfile.txt"));
        assert_eq!(escaped, "test\\nfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(Path::new("test\rfile.txt"));
        assert_eq!(escaped, "test\\rfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(Path::new("test\\file.txt"));
        assert_eq!(escaped, "test\\\\file.txt");
        assert_eq!(prefix, "\\");
    }

    #[test]
    fn test_calculate_blake2b_length() {
        assert_eq!(calculate_blake2b_length(0).unwrap(), None);
        assert!(calculate_blake2b_length(10).is_err());
        assert!(calculate_blake2b_length(520).is_err());
        assert_eq!(calculate_blake2b_length(512).unwrap(), None);
        assert_eq!(calculate_blake2b_length(256).unwrap(), Some(32));
    }

    #[test]
    fn test_detect_algo() {
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SYSV, None).unwrap().name,
            ALGORITHM_OPTIONS_SYSV
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_BSD, None).unwrap().name,
            ALGORITHM_OPTIONS_BSD
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_CRC, None).unwrap().name,
            ALGORITHM_OPTIONS_CRC
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_MD5, None).unwrap().name,
            ALGORITHM_OPTIONS_MD5
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA1, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA1
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA224, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA224
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA256, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA256
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA384, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA384
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA512, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA512
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_BLAKE2B, None).unwrap().name,
            ALGORITHM_OPTIONS_BLAKE2B
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_BLAKE3, None).unwrap().name,
            ALGORITHM_OPTIONS_BLAKE3
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SM3, None).unwrap().name,
            ALGORITHM_OPTIONS_SM3
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHAKE128, Some(128))
                .unwrap()
                .name,
            ALGORITHM_OPTIONS_SHAKE128
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHAKE256, Some(256))
                .unwrap()
                .name,
            ALGORITHM_OPTIONS_SHAKE256
        );
        assert_eq!(detect_algo("sha3_224", Some(224)).unwrap().name, "SHA3_224");
        assert_eq!(detect_algo("sha3_256", Some(256)).unwrap().name, "SHA3_256");
        assert_eq!(detect_algo("sha3_384", Some(384)).unwrap().name, "SHA3_384");
        assert_eq!(detect_algo("sha3_512", Some(512)).unwrap().name, "SHA3_512");

        assert!(detect_algo("sha3_512", None).is_err());
    }

    #[test]
    fn test_algo_based_regex() {
        let algo_based_regex = Regex::new(ALGO_BASED_REGEX).unwrap();
        #[allow(clippy::type_complexity)]
        let test_cases: &[(&[u8], Option<(&[u8], Option<&[u8]>, &[u8], &[u8])>)] = &[
            (b"SHA256 (example.txt) = d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2", Some((b"SHA256", None, b"example.txt", b"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2"))),
            // cspell:disable-next-line
            (b"BLAKE2b-512 (file) = abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef", Some((b"BLAKE2b", Some(b"512"), b"file", b"abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef"))),
            (b" MD5 (test) = 9e107d9d372bb6826bd81d3542a419d6", Some((b"MD5", None, b"test", b"9e107d9d372bb6826bd81d3542a419d6"))),
            (b"SHA-1 (anotherfile) = a9993e364706816aba3e25717850c26c9cd0d89d", Some((b"SHA", Some(b"1"), b"anotherfile", b"a9993e364706816aba3e25717850c26c9cd0d89d"))),
        ];

        for (input, expected) in test_cases {
            let captures = algo_based_regex.captures(input);
            match expected {
                Some((algo, bits, filename, checksum)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(&captures.name("algo").unwrap().as_bytes(), algo);
                    assert_eq!(&captures.name("bits").map(|m| m.as_bytes()), bits);
                    assert_eq!(&captures.name("filename").unwrap().as_bytes(), filename);
                    assert_eq!(&captures.name("checksum").unwrap().as_bytes(), checksum);
                }
                None => {
                    assert!(captures.is_none());
                }
            }
        }
    }

    #[test]
    fn test_double_space_regex() {
        let double_space_regex = Regex::new(DOUBLE_SPACE_REGEX).unwrap();

        #[allow(clippy::type_complexity)]
        let test_cases: &[(&[u8], Option<(&[u8], &[u8])>)] = &[
            (
                b"60b725f10c9c85c70d97880dfe8191b3  a",
                Some((b"60b725f10c9c85c70d97880dfe8191b3", b"a")),
            ),
            (
                b"bf35d7536c785cf06730d5a40301eba2   b",
                Some((b"bf35d7536c785cf06730d5a40301eba2", b" b")),
            ),
            (
                b"f5b61709718c1ecf8db1aea8547d4698  *c",
                Some((b"f5b61709718c1ecf8db1aea8547d4698", b"*c")),
            ),
            (
                b"b064a020db8018f18ff5ae367d01b212  dd",
                Some((b"b064a020db8018f18ff5ae367d01b212", b"dd")),
            ),
            (
                b"b064a020db8018f18ff5ae367d01b212   ",
                Some((b"b064a020db8018f18ff5ae367d01b212", b" ")),
            ),
            (b"invalidchecksum  test", None),
        ];

        for (input, expected) in test_cases {
            let captures = double_space_regex.captures(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(&captures.name("checksum").unwrap().as_bytes(), checksum);
                    assert_eq!(&captures.name("filename").unwrap().as_bytes(), filename);
                }
                None => {
                    assert!(captures.is_none());
                }
            }
        }
    }

    #[test]
    fn test_single_space_regex() {
        let single_space_regex = Regex::new(SINGLE_SPACE_REGEX).unwrap();
        #[allow(clippy::type_complexity)]
        let test_cases: &[(&[u8], Option<(&[u8], &[u8])>)] = &[
            (
                b"60b725f10c9c85c70d97880dfe8191b3 a",
                Some((b"60b725f10c9c85c70d97880dfe8191b3", b"a")),
            ),
            (
                b"bf35d7536c785cf06730d5a40301eba2 b",
                Some((b"bf35d7536c785cf06730d5a40301eba2", b"b")),
            ),
            (
                b"f5b61709718c1ecf8db1aea8547d4698 *c",
                Some((b"f5b61709718c1ecf8db1aea8547d4698", b"*c")),
            ),
            (
                b"b064a020db8018f18ff5ae367d01b212 dd",
                Some((b"b064a020db8018f18ff5ae367d01b212", b"dd")),
            ),
            (b"invalidchecksum test", None),
        ];

        for (input, expected) in test_cases {
            let captures = single_space_regex.captures(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(&captures.name("checksum").unwrap().as_bytes(), checksum);
                    assert_eq!(&captures.name("filename").unwrap().as_bytes(), filename);
                }
                None => {
                    assert!(captures.is_none());
                }
            }
        }
    }

    #[test]
    fn test_determine_regex() {
        // Test algo-based regex
        let line_algo_based =
            OsString::from("MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e");
        let (regex, algo_based) = determine_regex(&line_algo_based).unwrap();
        assert!(algo_based);
        assert!(regex.is_match(os_str_as_bytes(&line_algo_based).unwrap()));

        // Test double-space regex
        let line_double_space = OsString::from("d41d8cd98f00b204e9800998ecf8427e  example.txt");
        let (regex, algo_based) = determine_regex(&line_double_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(os_str_as_bytes(&line_double_space).unwrap()));

        // Test single-space regex
        let line_single_space = OsString::from("d41d8cd98f00b204e9800998ecf8427e example.txt");
        let (regex, algo_based) = determine_regex(&line_single_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(os_str_as_bytes(&line_single_space).unwrap()));

        // Test invalid checksum line
        let line_invalid = OsString::from("invalid checksum line");
        assert!(determine_regex(&line_invalid).is_none());

        // Test leading space before checksum line
        let line_algo_based_leading_space =
            OsString::from("   MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e");
        let res = determine_regex(&line_algo_based_leading_space);
        assert!(res.is_some());
        assert_eq!(res.unwrap().0.as_str(), ALGO_BASED_REGEX);

        // Test trailing space after checksum line (should fail)
        let line_algo_based_leading_space =
            OsString::from("MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e ");
        let res = determine_regex(&line_algo_based_leading_space);
        assert!(res.is_none());
    }

    #[test]
    fn test_get_expected_digest() {
        let re = Regex::new(ALGO_BASED_REGEX_BASE64).unwrap();
        let caps = re
            .captures(b"SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=")
            .unwrap();

        let result = get_expected_digest_as_hex_string(&caps, &re);

        assert_eq!(
            result.unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_get_expected_checksum_invalid() {
        let re = Regex::new(ALGO_BASED_REGEX_BASE64).unwrap();
        let caps = re
            .captures(b"SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU")
            .unwrap();

        let result = get_expected_digest_as_hex_string(&caps, &re);

        assert!(result.is_none());
    }

    #[test]
    fn test_print_file_report() {
        let opts = ChecksumOptions::default();

        let cases: &[(&[u8], FileChecksumResult, &str, &[u8])] = &[
            (b"filename", FileChecksumResult::Ok, "", b"filename: OK\n"),
            (
                b"filename",
                FileChecksumResult::Failed,
                "",
                b"filename: FAILED\n",
            ),
            (
                b"filename",
                FileChecksumResult::CantOpen,
                "",
                b"filename: FAILED open or read\n",
            ),
            (
                b"filename",
                FileChecksumResult::Ok,
                "prefix",
                b"prefixfilename: OK\n",
            ),
            (
                b"funky\xffname",
                FileChecksumResult::Ok,
                "",
                b"funky\xffname: OK\n",
            ),
        ];

        for (filename, result, prefix, expected) in cases {
            let mut buffer: Vec<u8> = vec![];
            print_file_report(&mut buffer, filename, *result, prefix, opts);
            assert_eq!(&buffer, expected)
        }
    }
}
