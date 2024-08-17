// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore anotherfile invalidchecksum regexes JWZG

use data_encoding::BASE64;
use os_display::Quotable;
use regex::bytes::{Captures, Regex};
#[cfg(not(unix))]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader, Read},
    str,
};
use utils::{get_filename_for_output, unescape_filename};

use crate::{
    error::{set_exit_code, FromIo, UError, UResult, USimpleError},
    os_str_as_bytes, read_os_string_lines, show, show_error, show_warning_caps,
    sum::{Digest, DigestWriter},
    util_name,
};
use std::fmt::Write;
use std::io::stdin;
use thiserror::Error;

pub mod algo;
mod utils;
pub use utils::escape_filename; // Used in hashsum.rs

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

/// This struct regroups CLI flags.
#[derive(Debug, Clone, Copy)]
pub struct ChecksumOptions {
    pub binary: bool,
    pub ignore_missing: bool,
    pub quiet: bool,
    pub status: bool,
    pub strict: bool,
    pub warn: bool,
}

#[derive(Default)]
struct ChecksumResult {
    pub bad_format: i32,
    pub failed_cksum: i32,
    pub failed_open_file: i32,
}

impl ChecksumResult {
    /// Print diagnostic lines at the end of the processing of a checksum file.
    #[allow(clippy::comparison_chain)]
    fn print_output(&self, ignore_missing: bool, status: bool) {
        if self.bad_format == 1 {
            show_warning_caps!("{} line is improperly formatted", self.bad_format);
        } else if self.bad_format > 1 {
            show_warning_caps!("{} lines are improperly formatted", self.bad_format);
        }

        if !status {
            if self.failed_cksum == 1 {
                show_warning_caps!("{} computed checksum did NOT match", self.failed_cksum);
            } else if self.failed_cksum > 1 {
                show_warning_caps!("{} computed checksums did NOT match", self.failed_cksum);
            }
        }
        if !ignore_missing {
            if self.failed_open_file == 1 {
                show_warning_caps!("{} listed file could not be read", self.failed_open_file);
            } else if self.failed_open_file > 1 {
                show_warning_caps!("{} listed files could not be read", self.failed_open_file);
            }
        }
    }
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
    #[error("{filename}: no properly formatted checksum lines found")]
    NoProperlyFormattedChecksumLinesFound { filename: String },
}

impl UError for ChecksumError {
    fn code(&self) -> i32 {
        1
    }
}

enum LineCheckError {
    UError(Box<dyn UError>),
    // ImproperlyFormatted,
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

#[allow(clippy::enum_variant_names)]
enum FileCheckError {
    UError(Box<dyn UError>),
    NonCriticalError,
    CriticalError,
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

// Regexp to handle the three input formats:
// 1. <algo>[-<bits>] (<filename>) = <checksum>
//    algo must be uppercase or b (for blake2b)
// 2. <checksum> [* ]<filename>
// 3. <checksum> [*]<filename> (only one space)
const ALGO_BASED_REGEX: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>.*)\)\s*=\s*(?P<checksum>[a-fA-F0-9]+)$";
const ALGO_BASED_REGEX_BASE64: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>.*)\)\s*=\s*(?P<checksum>[A-Za-z0-9+/]+={0,2})$";

const DOUBLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s{2}(?P<filename>.*)$";

// In this case, we ignore the *
const SINGLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s(?P<filename>\*?.*)$";

/// Determines the appropriate regular expression to use based on the provided lines.
fn determine_regex<S: AsRef<OsStr>>(lines: &[S]) -> Option<(Regex, bool)> {
    let regexes = [
        (Regex::new(ALGO_BASED_REGEX).unwrap(), true),
        (Regex::new(DOUBLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(SINGLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(ALGO_BASED_REGEX_BASE64).unwrap(), true),
    ];

    for line in lines {
        let line_trim = os_str_as_bytes(line.as_ref()).expect("UTF-8 decoding failed");

        for (regex, is_algo_based) in &regexes {
            if regex.is_match(line_trim) {
                return Some((regex.clone(), *is_algo_based));
            }
        }
    }

    None
}

// Converts bytes to a hexadecimal string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        })
}

fn get_base64_checksum(checksum: &[u8]) -> Option<String> {
    match BASE64.decode(checksum) {
        Ok(decoded_bytes) => {
            match str::from_utf8(&decoded_bytes) {
                Ok(decoded_str) => Some(decoded_str.to_string()),
                Err(_) => Some(bytes_to_hex(&decoded_bytes)), // Handle as raw bytes if not valid UTF-8
            }
        }
        Err(_) => None,
    }
}

fn get_expected_checksum(filename: &str, checksum: &[u8], chosen_regex: &Regex) -> UResult<String> {
    if chosen_regex.as_str() == ALGO_BASED_REGEX_BASE64 {
        get_base64_checksum(checksum).ok_or(Box::new(
            ChecksumError::NoProperlyFormattedChecksumLinesFound {
                filename: (&filename).to_string(),
            },
        ))
    } else {
        // Assume the checksum is already valid UTF-8
        // (Validation should have been handled by regex)
        Ok(str::from_utf8(checksum).unwrap().to_string())
    }
}

/// Returns a reader that reads from the specified file, or from stdin if `filename_to_check` is "-".
fn get_file_to_check(
    filename: &OsStr,
    ignore_missing: bool,
    res: &mut ChecksumResult,
) -> Option<Box<dyn Read>> {
    let filename_lossy = String::from_utf8_lossy(os_str_as_bytes(filename).expect("UTF-8 error"));
    if filename == "-" {
        Some(Box::new(stdin())) // Use stdin if "-" is specified in the checksum file
    } else {
        match File::open(filename) {
            Ok(f) => {
                if f.metadata().ok()?.is_dir() {
                    show!(USimpleError::new(
                        1,
                        format!("{filename_lossy}: Is a directory")
                    ));
                    None
                } else {
                    Some(Box::new(f))
                }
            }
            Err(err) => {
                if !ignore_missing {
                    // yes, we have both stderr and stdout here
                    show!(err.map_err_context(|| filename_lossy.to_string()));
                    println!("{filename_lossy}: FAILED open or read");
                }
                res.failed_open_file += 1;
                // we could not open the file but we want to continue
                None
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
    res: &mut ChecksumResult,
    properly_formatted: &mut bool,
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
        res.bad_format += 1;
        *properly_formatted = false;
        return None;
    }

    if !SUPPORTED_ALGORITHMS.contains(&algorithm.as_str()) {
        // Not supported algo, leave early
        *properly_formatted = false;
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
            *properly_formatted = false;
            None // Return None to signal a divisibility issue
        }
    })?;

    Some((algorithm, bits))
}

#[allow(clippy::too_many_arguments)]
fn process_checksum_line(
    filename_input: &OsStr,
    line: &OsStr,
    i: usize,
    chosen_regex: &Regex,
    is_algo_based_format: bool,
    res: &mut ChecksumResult,
    cli_algo_name: Option<&str>,
    cli_algo_length: Option<usize>,
    properly_formatted: &mut bool,
    correct_format: &mut usize,
    opts: ChecksumOptions,
) -> Result<(), LineCheckError> {
    if let Some(caps) =
        chosen_regex.captures(os_str_as_bytes(line).expect("UTF-8 decoding failure"))
    {
        *properly_formatted = true;

        let checksum = caps
            .name("checksum")
            .expect("safe because of regex")
            .as_bytes();
        let mut filename_to_check = caps
            .name("filename")
            .expect("safe because of regex")
            .as_bytes();

        if filename_to_check.starts_with(b"*")
            && i == 0
            && chosen_regex.as_str() == SINGLE_SPACE_REGEX
        {
            // Remove the leading asterisk if present - only for the first line
            filename_to_check = &filename_to_check[1..];
        }

        let filename_lossy = String::from_utf8_lossy(filename_to_check);
        let expected_checksum = get_expected_checksum(&filename_lossy, checksum, &chosen_regex)
            .map_err(LineCheckError::from)?;

        // If the algo_name is provided, we use it, otherwise we try to detect it
        let (algo_name, length) = if is_algo_based_format {
            identify_algo_name_and_length(&caps, cli_algo_name, res, properly_formatted)
                .unwrap_or((String::new(), None))
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
            (String::new(), None)
        };

        if algo_name.is_empty() {
            // we haven't been able to detect the algo name. No point to continue
            *properly_formatted = false;

            // FIXME(dprn): report error in some way ?
            return Ok(());
        }
        let mut algo = algo::detect_algo(&algo_name, length)?;

        let (filename_to_check_unescaped, prefix) = unescape_filename(filename_to_check);

        #[cfg(unix)]
        let real_filename_to_check = OsStr::from_bytes(&filename_to_check_unescaped);
        #[cfg(not(unix))]
        let real_filename_to_check =
            &OsString::from(String::from_utf8(filename_to_check_unescaped).unwrap());

        // manage the input file
        let file_to_check =
            match get_file_to_check(real_filename_to_check, opts.ignore_missing, res) {
                Some(file) => file,
                None => {
                    // FIXME(dprn): report error in some way ?
                    return Ok(());
                }
            };
        let mut file_reader = BufReader::new(file_to_check);
        // Read the file and calculate the checksum
        let create_fn = &mut algo.create_fn;
        let mut digest = create_fn();
        let (calculated_checksum, _) =
            digest_reader(&mut digest, &mut file_reader, opts.binary, algo.bits).unwrap();

        // Do the checksum validation
        if expected_checksum == calculated_checksum {
            if !opts.quiet && !opts.status {
                println!("{prefix}{filename_lossy}: OK");
            }
            *correct_format += 1;
        } else {
            if !opts.status {
                println!("{prefix}{filename_lossy}: FAILED");
            }
            res.failed_cksum += 1;
        }
    } else {
        if line.is_empty() {
            // Don't show any warning for empty lines
            // FIXME(dprn): report error in some way ?
            return Ok(());
        }
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

        res.bad_format += 1;
    }
    Ok(())
}

fn process_checksum_file(
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
    filename_input: &OsStr,
    opts: ChecksumOptions,
) -> Result<(), FileCheckError> {
    let mut correct_format = 0;
    let mut properly_formatted = false;
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
                return Err(FileCheckError::NonCriticalError);
            }
        }
    };

    let reader = BufReader::new(file);
    let lines = read_os_string_lines(reader).collect::<Vec<_>>();

    let Some((chosen_regex, is_algo_based_format)) = determine_regex(&lines) else {
        let e = ChecksumError::NoProperlyFormattedChecksumLinesFound {
            filename: get_filename_for_output(filename_input, input_is_stdin),
        };
        show_error!("{e}");
        set_exit_code(1);
        return Err(FileCheckError::NonCriticalError);
    };

    for (i, line) in lines.iter().enumerate() {
        use LineCheckError::*;
        match process_checksum_line(
            filename_input,
            line,
            i,
            &chosen_regex,
            is_algo_based_format,
            &mut res,
            algo_name_input,
            length_input,
            &mut properly_formatted,
            &mut correct_format,
            opts,
        ) {
            Err(UError(e)) => return Err(e.into()),
            // Err(_) => todo!(),
            Ok(_) => (),
        }
    }

    // not a single line correctly formatted found
    // return an error
    if !properly_formatted {
        if !opts.status {
            return Err(ChecksumError::NoProperlyFormattedChecksumLinesFound {
                filename: get_filename_for_output(filename_input, input_is_stdin),
            }
            .into());
        }
        set_exit_code(1);

        return Err(FileCheckError::CriticalError);
    }

    if opts.ignore_missing && correct_format == 0 {
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

    // if any incorrectly formatted line, show it
    res.print_output(opts.ignore_missing, opts.status);

    Ok(())
}

/// Do the checksum validation (can be strict or not)
///
pub fn perform_checksum_validation<'a, I>(
    files: I,
    opts: ChecksumOptions,
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        use FileCheckError::*;
        match process_checksum_file(algo_name_input, length_input, filename_input, opts) {
            Err(UError(e)) => return Err(e),
            Err(CriticalError) => break,
            Err(NonCriticalError) | Ok(_) => continue,
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn test_calculate_blake2b_length() {
        assert_eq!(calculate_blake2b_length(0).unwrap(), None);
        assert!(calculate_blake2b_length(10).is_err());
        assert!(calculate_blake2b_length(520).is_err());
        assert_eq!(calculate_blake2b_length(512).unwrap(), None);
        assert_eq!(calculate_blake2b_length(256).unwrap(), Some(32));
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
            let captures = algo_based_regex.captures(*input);
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
        let lines_algo_based = ["MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e"]
            .iter()
            .map(|s| OsString::from(s.to_string()))
            .collect::<Vec<_>>();
        let (regex, algo_based) = determine_regex(&lines_algo_based).unwrap();
        assert!(algo_based);
        assert!(regex.is_match(os_str_as_bytes(&lines_algo_based[0]).unwrap()));

        // Test double-space regex
        let lines_double_space = ["d41d8cd98f00b204e9800998ecf8427e  example.txt"]
            .iter()
            .map(|s| OsString::from(s.to_string()))
            .collect::<Vec<_>>();
        let (regex, algo_based) = determine_regex(&lines_double_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(os_str_as_bytes(&lines_double_space[0]).unwrap()));

        // Test single-space regex
        let lines_single_space = ["d41d8cd98f00b204e9800998ecf8427e example.txt"]
            .iter()
            .map(|s| OsString::from(s.to_string()))
            .collect::<Vec<_>>();
        let (regex, algo_based) = determine_regex(&lines_single_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(os_str_as_bytes(&lines_single_space[0]).unwrap()));

        // Test double-space regex start with invalid
        let lines_double_space = ["ERR", "d41d8cd98f00b204e9800998ecf8427e  example.txt"]
            .iter()
            .map(|s| OsString::from(s.to_string()))
            .collect::<Vec<_>>();
        let (regex, algo_based) = determine_regex(&lines_double_space).unwrap();
        assert!(!algo_based);
        assert!(!regex.is_match(os_str_as_bytes(&lines_double_space[0]).unwrap()));
        assert!(regex.is_match(os_str_as_bytes(&lines_double_space[1]).unwrap()));

        // Test invalid checksum line
        let lines_invalid = ["invalid checksum line"]
            .iter()
            .map(|s| OsString::from(s.to_string()))
            .collect::<Vec<_>>();
        assert!(determine_regex(&lines_invalid).is_none());
    }

    #[test]
    fn test_get_expected_checksum() {
        let re = Regex::new(ALGO_BASED_REGEX_BASE64).unwrap();
        let caps = re
            .captures(b"SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=")
            .unwrap();
        let checksum = caps.name("checksum").unwrap().as_bytes();

        let result = get_expected_checksum("filename", checksum, &re);

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
        let checksum = caps.name("checksum").unwrap().as_bytes();

        let result = get_expected_checksum("filename", checksum, &re);

        assert!(result.is_err());
    }
}
