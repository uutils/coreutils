// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore rsplit hexdigit bitlen invalidchecksum inva idchecksum xffname

use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader, Read, Write, stdin};

use os_display::Quotable;

use crate::checksum::{AlgoKind, ChecksumError, SizedAlgoKind, digest_reader, unescape_filename};
use crate::error::{FromIo, UError, UResult, USimpleError};
use crate::quoting_style::{QuotingStyle, locale_aware_escape_name};
use crate::sum::DigestOutput;
use crate::{
    os_str_as_bytes, os_str_from_bytes, read_os_string_lines, show, show_error, show_warning_caps,
    translate,
};

/// To what level should checksum validation print logging info.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy, Default)]
pub enum ChecksumVerbose {
    Status,
    Quiet,
    #[default]
    Normal,
    Warning,
}

impl ChecksumVerbose {
    pub fn new(status: bool, quiet: bool, warn: bool) -> Self {
        use ChecksumVerbose::*;

        // Assume only one of the three booleans will be enabled at once.
        // This is ensured by clap's overriding arguments.
        match (status, quiet, warn) {
            (true, _, _) => Status,
            (_, true, _) => Quiet,
            (_, _, true) => Warning,
            _ => Normal,
        }
    }

    #[inline]
    pub fn over_status(self) -> bool {
        self > Self::Status
    }

    #[inline]
    pub fn over_quiet(self) -> bool {
        self > Self::Quiet
    }

    #[inline]
    pub fn at_least_warning(self) -> bool {
        self >= Self::Warning
    }
}

/// This struct regroups CLI flags.
#[derive(Debug, Default, Clone, Copy)]
pub struct ChecksumValidateOptions {
    pub ignore_missing: bool,
    pub strict: bool,
    pub verbose: ChecksumVerbose,
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
    /// reading of the checksum file failed
    CantOpenChecksumFile,
    /// processing of the file is considered as a failure regarding the
    /// provided flags. This however does not stop the processing of
    /// further files.
    Failed,
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

fn print_cksum_report(res: &ChecksumResult) {
    if res.bad_format > 0 {
        show_warning_caps!(
            "{}",
            translate!("checksum-bad-format", "count" => res.bad_format)
        );
    }

    if res.failed_cksum > 0 {
        show_warning_caps!(
            "{}",
            translate!("checksum-failed-cksum", "count" => res.failed_cksum)
        );
    }

    if res.failed_open_file > 0 {
        show_warning_caps!(
            "{}",
            translate!("checksum-failed-open-file", "count" => res.failed_open_file)
        );
    }
}

/// Print a "no properly formatted lines" message in stderr
#[inline]
fn log_no_properly_formatted(filename: impl Display) {
    show_error!(
        "{}",
        translate!("checksum-no-properly-formatted", "checksum_file" => filename)
    );
}

/// Print a "no file was verified" message in stderr
#[inline]
fn log_no_file_verified(filename: impl Display) {
    show_error!(
        "{}",
        translate!("checksum-no-file-verified", "checksum_file" => filename)
    );
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
            Self::Ok
        } else {
            Self::Failed
        }
    }

    /// The cli options might prevent to display on the outcome of the
    /// comparison on STDOUT.
    fn can_display(&self, verbose: ChecksumVerbose) -> bool {
        match self {
            Self::Ok => verbose.over_quiet(),
            Self::Failed => verbose.over_status(),
            Self::CantOpen => true,
        }
    }
}

impl Display for FileChecksumResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Failed => write!(f, "FAILED"),
            Self::CantOpen => write!(f, "FAILED open or read"),
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
    verbose: ChecksumVerbose,
) {
    if result.can_display(verbose) {
        let _ = write!(w, "{prefix}");
        let _ = w.write_all(filename);
        let _ = writeln!(w, ": {result}");
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum LineFormat {
    AlgoBased,
    SingleSpace,
    Untagged,
}

impl LineFormat {
    /// parse [tagged output format]
    /// Normally the format is simply space separated but openssl does not
    /// respect the gnu definition.
    ///
    /// [tagged output format]: https://www.gnu.org/software/coreutils/manual/html_node/cksum-output-modes.html#cksum-output-modes-1
    fn parse_algo_based(line: &[u8]) -> Option<LineInfo> {
        //   r"\MD5 (a\\ b) = abc123",
        //   BLAKE2b(44)= a45a4c4883cce4b50d844fab460414cc2080ca83690e74d850a9253e757384366382625b218c8585daee80f34dc9eb2f2fde5fb959db81cd48837f9216e7b0fa
        let trimmed = line.trim_ascii_start();
        let algo_start = usize::from(trimmed.starts_with(b"\\"));
        let rest = &trimmed[algo_start..];

        enum SubCase {
            Posix,
            OpenSSL,
        }
        // find the next parenthesis  using byte search (not next whitespace) because openssl's
        // tagged format does not put a space before (filename)

        let par_idx = rest.iter().position(|&b| b == b'(')?;
        let sub_case = if rest[par_idx - 1] == b' ' {
            SubCase::Posix
        } else {
            SubCase::OpenSSL
        };

        let algo_substring = match sub_case {
            SubCase::Posix => &rest[..par_idx - 1],
            SubCase::OpenSSL => &rest[..par_idx],
        };
        let mut algo_parts = algo_substring.splitn(2, |&b| b == b'-');
        let algo = algo_parts.next()?;

        // Parse algo_bits if present
        let algo_bits = algo_parts
            .next()
            .and_then(|s| std::str::from_utf8(s).ok()?.parse::<usize>().ok());

        // Check algo format: uppercase ASCII or digits or "BLAKE2b"
        let is_valid_algo = algo == b"BLAKE2b"
            || algo
                .iter()
                .all(|&b| b.is_ascii_uppercase() || b.is_ascii_digit());
        if !is_valid_algo {
            return None;
        }
        // SAFETY: we just validated the contents of algo, we can unsafely make a
        // String from it
        let algo_utf8 = unsafe { String::from_utf8_unchecked(algo.to_vec()) };
        // stripping '(' not ' (' since we matched on ( not whitespace because of openssl.
        let after_paren = rest.get(par_idx + 1..)?;
        let (filename, checksum) = match sub_case {
            SubCase::Posix => ByteSliceExt::rsplit_once(after_paren, b") = ")?,
            SubCase::OpenSSL => ByteSliceExt::rsplit_once(after_paren, b")= ")?,
        };

        let checksum_utf8 = Self::validate_checksum_format(checksum)?;

        Some(LineInfo {
            algo_name: Some(algo_utf8),
            algo_bit_len: algo_bits,
            checksum: checksum_utf8,
            filename: filename.to_vec(),
            format: Self::AlgoBased,
        })
    }

    #[allow(rustdoc::invalid_html_tags)]
    /// parse [untagged output format]
    /// The format is simple, either "<checksum>  <filename>" or
    /// "<checksum> *<filename>"
    ///
    /// [untagged output format]: https://www.gnu.org/software/coreutils/manual/html_node/cksum-output-modes.html#cksum-output-modes-1
    fn parse_untagged(line: &[u8]) -> Option<LineInfo> {
        let space_idx = line.iter().position(|&b| b == b' ')?;
        let checksum = &line[..space_idx];

        let checksum_utf8 = Self::validate_checksum_format(checksum)?;

        let rest = &line[space_idx..];
        let filename = rest
            .strip_prefix(b"  ")
            .or_else(|| rest.strip_prefix(b" *"))?;

        Some(LineInfo {
            algo_name: None,
            algo_bit_len: None,
            checksum: checksum_utf8,
            filename: filename.to_vec(),
            format: Self::Untagged,
        })
    }

    #[allow(rustdoc::invalid_html_tags)]
    /// parse [untagged output format]
    /// Normally the format is simple, either "<checksum>  <filename>" or
    /// "<checksum> *<filename>"
    /// But the bsd tests expect special single space behavior where
    /// checksum and filename are separated only by a space, meaning the second
    /// space or asterisk is part of the file name.
    /// This parser accounts for this variation
    ///
    /// [untagged output format]: https://www.gnu.org/software/coreutils/manual/html_node/cksum-output-modes.html#cksum-output-modes-1
    fn parse_single_space(line: &[u8]) -> Option<LineInfo> {
        // Find first space
        let space_idx = line.iter().position(|&b| b == b' ')?;
        let checksum = &line[..space_idx];
        if !checksum.iter().all(|&b| b.is_ascii_hexdigit()) || checksum.is_empty() {
            return None;
        }
        // SAFETY: we just validated the contents of checksum, we can unsafely make a
        // String from it
        let checksum_utf8 = unsafe { String::from_utf8_unchecked(checksum.to_vec()) };

        let filename = line.get(space_idx + 1..)?; // Skip single space

        Some(LineInfo {
            algo_name: None,
            algo_bit_len: None,
            checksum: checksum_utf8,
            filename: filename.to_vec(),
            format: Self::SingleSpace,
        })
    }

    /// Ensure that the given checksum is syntactically valid (that it is either
    /// hexadecimal or base64 encoded).
    fn validate_checksum_format(checksum: &[u8]) -> Option<String> {
        if checksum.is_empty() {
            return None;
        }

        let mut is_base64 = false;

        for index in 0..checksum.len() {
            match checksum[index..] {
                // ASCII alphanumeric
                [b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9', ..] => (),
                // Base64 special character
                [b'+' | b'/', ..] => is_base64 = true,
                // Base64 end of string padding
                [b'='] | [b'=', b'='] | [b'=', b'=', b'='] => {
                    is_base64 = true;
                    break;
                }
                // Any other character means the checksum is wrong
                _ => return None,
            }
        }

        // If base64 characters were encountered, make sure the checksum has a
        // length multiple of 4.
        //
        // This check is not enough because it may allow base64-encoded
        // checksums that are fully alphanumeric. Another check happens later
        // when we are provided with a length hint to detect ambiguous
        // base64-encoded checksums.
        if is_base64 && checksum.len() % 4 != 0 {
            return None;
        }

        // SAFETY: we just validated the contents of checksum, we can unsafely make a
        // String from it
        Some(unsafe { String::from_utf8_unchecked(checksum.to_vec()) })
    }
}

// Helper trait for byte slice operations
trait ByteSliceExt {
    /// Look for a pattern from right to left, return surrounding parts if found.
    fn rsplit_once(&self, pattern: &[u8]) -> Option<(&Self, &Self)>;
}

impl ByteSliceExt for [u8] {
    fn rsplit_once(&self, pattern: &[u8]) -> Option<(&Self, &Self)> {
        let pos = self
            .windows(pattern.len())
            .rev()
            .position(|w| w == pattern)?;
        Some((
            &self[..self.len() - pattern.len() - pos],
            &self[self.len() - pos..],
        ))
    }
}

/// Hold the data extracted from a checksum line.
struct LineInfo {
    algo_name: Option<String>,
    algo_bit_len: Option<usize>,
    checksum: String,
    filename: Vec<u8>,
    format: LineFormat,
}

impl LineInfo {
    /// Returns a `LineInfo` parsed from a checksum line.
    /// The function will run 3 parsers against the line and select the first one that matches
    /// to populate the fields of the struct.
    /// However, there is a catch to handle regarding the handling of `cached_line_format`.
    /// In case of non-algo-based format, if `cached_line_format` is Some, it must take the priority
    /// over the detected format. Otherwise, we must set it the the detected format.
    /// This specific behavior is emphasized by the test
    /// `test_hashsum::test_check_md5sum_only_one_space`.
    fn parse(s: impl AsRef<OsStr>, cached_line_format: &mut Option<LineFormat>) -> Option<Self> {
        let line_bytes = os_str_as_bytes(s.as_ref()).ok()?;

        if let Some(info) = LineFormat::parse_algo_based(line_bytes) {
            return Some(info);
        }
        if let Some(cached_format) = cached_line_format {
            match cached_format {
                LineFormat::Untagged => LineFormat::parse_untagged(line_bytes),
                LineFormat::SingleSpace => LineFormat::parse_single_space(line_bytes),
                LineFormat::AlgoBased => unreachable!("we never catch the algo based format"),
            }
        } else if let Some(info) = LineFormat::parse_untagged(line_bytes) {
            *cached_line_format = Some(LineFormat::Untagged);
            Some(info)
        } else if let Some(info) = LineFormat::parse_single_space(line_bytes) {
            *cached_line_format = Some(LineFormat::SingleSpace);
            Some(info)
        } else {
            None
        }
    }
}

/// Extract the expected digest from the checksum string and decode it
fn get_raw_expected_digest(checksum: &str, byte_len_hint: Option<usize>) -> Option<Vec<u8>> {
    // If the length of the digest is not a multiple of 2, then it must be
    // improperly formatted (1 byte is 2 hex digits, and base64 strings should
    // always be a multiple of 4).
    if checksum.len() % 2 != 0 {
        return None;
    }

    let checks_hint = |len| byte_len_hint.is_none_or(|hint| hint == len);

    // If the length of the string matches the one to be expected (in case it's
    // given) AND the digest can be decoded as hexadecimal, just go with it.
    if checks_hint(checksum.len() / 2) {
        if let Ok(raw_ck) = hex::decode(checksum) {
            return Some(raw_ck);
        }
    }

    // If the checksum cannot be decoded as hexadecimal, interpret it as Base64
    // instead.

    // But first, verify the encoded checksum length, which should be a
    // multiple of 4.
    //
    // It is important to check it before trying to decode, because the
    // forgiving mode of decoding will ignore if padding characters '=' are
    // MISSING, but to match GNU's behavior, we must reject it.
    if checksum.len() % 4 != 0 {
        return None;
    }

    // Perform the decoding and be FORGIVING about it, to allow for checksums
    // with INVALID padding to still be decoded. This is enforced by
    // `test_untagged_base64_matching_tag` in `test_cksum.rs`

    base64_simd::forgiving_decode_to_vec(checksum.as_bytes())
        .ok()
        .filter(|raw| checks_hint(raw.len()))
}

/// Returns a reader that reads from the specified file, or from stdin if `filename_to_check` is "-".
fn get_file_to_check(
    filename: &OsStr,
    opts: ChecksumValidateOptions,
) -> Result<Box<dyn Read>, LineCheckError> {
    let filename_bytes = os_str_as_bytes(filename).map_err(|e| LineCheckError::UError(e.into()))?;

    if filename == "-" {
        Ok(Box::new(io::stdin())) // Use stdin if "-" is specified in the checksum file
    } else {
        let failed_open = || {
            print_file_report(
                io::stdout(),
                filename_bytes,
                FileChecksumResult::CantOpen,
                "",
                opts.verbose,
            );
        };
        let print_error = |err: io::Error| {
            show!(err.map_err_context(|| {
                locale_aware_escape_name(filename, QuotingStyle::SHELL_ESCAPE)
                    // This is non destructive thanks to the escaping
                    .to_string_lossy()
                    .to_string()
            }));
        };
        match File::open(filename) {
            Ok(f) => {
                if f.metadata()
                    .map_err(|_| LineCheckError::CantOpenFile)?
                    .is_dir()
                {
                    print_error(io::Error::new(
                        io::ErrorKind::IsADirectory,
                        "Is a directory",
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
                    print_error(err);
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
                Err(io::Error::other(
                    translate!("error-is-a-directory", "file" => filename.maybe_quote()),
                )
                .into())
            } else {
                Ok(Box::new(f))
            }
        }
        Err(_) => Err(io::Error::other(format!(
            "{}: {}",
            filename.maybe_quote(),
            translate!("error-file-not-found")
        ))
        .into()),
    }
}

/// Gets the algorithm name and length from the `LineInfo` if the algo-based format is matched.
fn identify_algo_name_and_length(
    line_info: &LineInfo,
    algo_name_input: Option<AlgoKind>,
    last_algo: &mut Option<String>,
) -> Result<(AlgoKind, Option<usize>), LineCheckError> {
    let algo_from_line = line_info.algo_name.clone().unwrap_or_default();
    let Ok(line_algo) = AlgoKind::from_cksum(algo_from_line.to_lowercase()) else {
        // Unknown algorithm
        return Err(LineCheckError::ImproperlyFormatted);
    };
    *last_algo = Some(algo_from_line);

    // check if we are called with XXXsum (example: md5sum) but we detected a
    // different algo parsing the file (for example SHA1 (f) = d...)
    //
    // Also handle the case cksum -s sm3 but the file contains other formats
    if let Some(algo_name_input) = algo_name_input {
        match (algo_name_input, line_algo) {
            (l, r) if l == r => (),
            // Edge case for SHA2, which matches SHA(224|256|384|512)
            (
                AlgoKind::Sha2,
                AlgoKind::Sha224 | AlgoKind::Sha256 | AlgoKind::Sha384 | AlgoKind::Sha512,
            ) => (),
            _ => return Err(LineCheckError::ImproperlyFormatted),
        }
    }

    let bytes = if let Some(bitlen) = line_info.algo_bit_len {
        match line_algo {
            AlgoKind::Blake2b if bitlen % 8 == 0 => Some(bitlen / 8),
            AlgoKind::Sha2 | AlgoKind::Sha3 if [224, 256, 384, 512].contains(&bitlen) => {
                Some(bitlen)
            }
            // Either
            //  the algo based line is provided with a bit length
            //  with an algorithm that does not support it (only Blake2B does).
            //
            //  eg: MD5-128 (foo.txt) = fffffffff
            //          ^ This is illegal
            // OR
            //  the given length is wrong because it's not a multiple of 8.
            _ => return Err(LineCheckError::ImproperlyFormatted),
        }
    } else if line_algo == AlgoKind::Blake2b {
        // Default length with BLAKE2b,
        Some(64)
    } else {
        None
    };

    Ok((line_algo, bytes))
}

/// Given a filename and an algorithm, compute the digest and compare it with
/// the expected one.
fn compute_and_check_digest_from_file(
    filename: &[u8],
    expected_checksum: &[u8],
    algo: SizedAlgoKind,
    opts: ChecksumValidateOptions,
) -> Result<(), LineCheckError> {
    let (filename_to_check_unescaped, prefix) = unescape_filename(filename);
    let real_filename_to_check = os_str_from_bytes(&filename_to_check_unescaped)?;

    // Open the input file
    let file_to_check = get_file_to_check(&real_filename_to_check, opts)?;
    let mut file_reader = BufReader::new(file_to_check);

    // Read the file and calculate the checksum
    let mut digest = algo.create_digest();

    // TODO: improve function signature to use ReadingMode instead of binary bool
    // Set binary to false because --binary is not supported with --check
    let (calculated_checksum, _) =
        digest_reader(&mut digest, &mut file_reader, /* binary */ false).unwrap();

    // Do the checksum validation
    let checksum_correct = match calculated_checksum {
        DigestOutput::Vec(data) => data == expected_checksum,
        DigestOutput::Crc(n) => n.to_be_bytes() == expected_checksum,
        DigestOutput::U16(n) => n.to_be_bytes() == expected_checksum,
    };
    print_file_report(
        std::io::stdout(),
        filename,
        FileChecksumResult::from_bool(checksum_correct),
        prefix,
        opts.verbose,
    );

    if checksum_correct {
        Ok(())
    } else {
        Err(LineCheckError::DigestMismatch)
    }
}

/// Check a digest checksum with non-algo based pre-treatment.
fn process_algo_based_line(
    line_info: &LineInfo,
    cli_algo_kind: Option<AlgoKind>,
    opts: ChecksumValidateOptions,
    last_algo: &mut Option<String>,
) -> Result<(), LineCheckError> {
    let filename_to_check = line_info.filename.as_slice();

    let (algo_kind, algo_byte_len) =
        identify_algo_name_and_length(line_info, cli_algo_kind, last_algo)?;

    // If the digest bitlen is known, we can check the format of the expected
    // checksum with it.
    let digest_char_length_hint = match (algo_kind, algo_byte_len) {
        (AlgoKind::Blake2b, Some(byte_len)) => Some(byte_len),
        _ => None,
    };

    let expected_checksum = get_raw_expected_digest(&line_info.checksum, digest_char_length_hint)
        .ok_or(LineCheckError::ImproperlyFormatted)?;

    let algo = SizedAlgoKind::from_unsized(algo_kind, algo_byte_len)?;

    compute_and_check_digest_from_file(filename_to_check, &expected_checksum, algo, opts)
}

/// Check a digest checksum with non-algo based pre-treatment.
fn process_non_algo_based_line(
    line_number: usize,
    line_info: &LineInfo,
    cli_algo_kind: AlgoKind,
    cli_algo_length: Option<usize>,
    opts: ChecksumValidateOptions,
) -> Result<(), LineCheckError> {
    let mut filename_to_check = line_info.filename.as_slice();
    if filename_to_check.starts_with(b"*")
        && line_number == 0
        && line_info.format == LineFormat::SingleSpace
    {
        // Remove the leading asterisk if present - only for the first line
        filename_to_check = &filename_to_check[1..];
    }
    let expected_checksum = get_raw_expected_digest(&line_info.checksum, None)
        .ok_or(LineCheckError::ImproperlyFormatted)?;

    // When a specific algorithm name is input, use it and use the provided
    // bits except when dealing with blake2b, sha2 and sha3, where we will
    // detect the length.
    let (algo_kind, algo_byte_len) = match cli_algo_kind {
        AlgoKind::Blake2b => (AlgoKind::Blake2b, Some(expected_checksum.len())),
        algo @ (AlgoKind::Sha2 | AlgoKind::Sha3) => {
            // multiplication by 8 to get the number of bits
            (algo, Some(expected_checksum.len() * 8))
        }
        _ => (cli_algo_kind, cli_algo_length),
    };

    let algo = SizedAlgoKind::from_unsized(algo_kind, algo_byte_len)?;

    compute_and_check_digest_from_file(filename_to_check, &expected_checksum, algo, opts)
}

/// Parses a checksum line, detect the algorithm to use, read the file and produce
/// its digest, and compare it to the expected value.
///
/// Returns `Ok(bool)` if the comparison happened, bool indicates if the digest
/// matched the expected.
/// If the comparison didn't happen, return a `LineChecksumError`.
fn process_checksum_line(
    line: &OsStr,
    i: usize,
    cli_algo_name: Option<AlgoKind>,
    cli_algo_length: Option<usize>,
    opts: ChecksumValidateOptions,
    cached_line_format: &mut Option<LineFormat>,
    last_algo: &mut Option<String>,
) -> Result<(), LineCheckError> {
    let line_bytes = os_str_as_bytes(line).map_err(|e| LineCheckError::UError(Box::new(e)))?;

    // Early return on empty or commented lines.
    if line.is_empty() || line_bytes.starts_with(b"#") {
        return Err(LineCheckError::Skipped);
    }

    // Use `LineInfo` to extract the data of a line.
    // Then, depending on its format, apply a different pre-treatment.
    let Some(line_info) = LineInfo::parse(line, cached_line_format) else {
        return Err(LineCheckError::ImproperlyFormatted);
    };

    if line_info.format == LineFormat::AlgoBased {
        process_algo_based_line(&line_info, cli_algo_name, opts, last_algo)
    } else if let Some(cli_algo) = cli_algo_name {
        // If we match a non-algo based parser, we expect a cli argument
        // to give us the algorithm to use
        process_non_algo_based_line(i, &line_info, cli_algo, cli_algo_length, opts)
    } else {
        // We have no clue of what algorithm to use
        Err(LineCheckError::ImproperlyFormatted)
    }
}

fn process_checksum_file(
    filename_input: &OsStr,
    cli_algo_kind: Option<AlgoKind>,
    cli_algo_length: Option<usize>,
    opts: ChecksumValidateOptions,
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
                return Err(FileCheckError::CantOpenChecksumFile);
            }
        }
    };

    let reader = BufReader::new(file);
    let lines = read_os_string_lines(reader).collect::<Vec<_>>();

    // cached_line_format is used to ensure that several non algo-based checksum line
    // will use the same parser.
    let mut cached_line_format = None;
    // last_algo caches the algorithm used in the last line to print a warning
    // message for the current line if improperly formatted.
    // Behavior tested in gnu_cksum_c::test_warn
    let mut last_algo = None;

    for (i, line) in lines.iter().enumerate() {
        let line_result = process_checksum_line(
            line,
            i,
            cli_algo_kind,
            cli_algo_length,
            opts,
            &mut cached_line_format,
            &mut last_algo,
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
            Err(ImproperlyFormatted) => {
                res.bad_format += 1;

                if opts.verbose.at_least_warning() {
                    let algo = if let Some(algo_name_input) = cli_algo_kind {
                        algo_name_input.to_uppercase()
                    } else if let Some(algo) = &last_algo {
                        algo.as_str()
                    } else {
                        "Unknown algorithm"
                    };
                    show_error!(
                        "{}",
                        translate!("checksum-error-algo-bad-format", "file" => filename_input.maybe_quote(), "line" => i + 1, "algo" => algo)
                    );
                }
            }
            Err(CantOpenFile | FileIsDirectory) => res.failed_open_file += 1,
            Err(FileNotFound) if !opts.ignore_missing => res.failed_open_file += 1,
            _ => (),
        }
    }

    let filename_display = || {
        if input_is_stdin {
            "standard input".maybe_quote()
        } else {
            filename_input.maybe_quote()
        }
    };

    // not a single line correctly formatted found
    // return an error
    if res.total_properly_formatted() == 0 {
        if opts.verbose.over_status() {
            log_no_properly_formatted(filename_display());
        }
        return Err(FileCheckError::Failed);
    }

    // if any incorrectly formatted line, show it
    if opts.verbose.over_status() {
        print_cksum_report(&res);
    }

    if opts.ignore_missing && res.correct == 0 {
        // we have only bad format
        // and we had ignore-missing
        if opts.verbose.over_status() {
            log_no_file_verified(filename_display());
        }
        return Err(FileCheckError::Failed);
    }

    // strict means that we should have an exit code.
    if opts.strict && res.bad_format > 0 {
        return Err(FileCheckError::Failed);
    }

    // If a file was missing, return an error unless we explicitly ignore it.
    if res.failed_open_file > 0 && !opts.ignore_missing {
        return Err(FileCheckError::Failed);
    }

    // Obviously, if a checksum failed at some point, report the error.
    if res.failed_cksum > 0 {
        return Err(FileCheckError::Failed);
    }

    Ok(())
}

/// Do the checksum validation (can be strict or not)
pub fn perform_checksum_validation<'a, I>(
    files: I,
    algo_kind: Option<AlgoKind>,
    length_input: Option<usize>,
    opts: ChecksumValidateOptions,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut failed = false;

    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        use FileCheckError::*;
        match process_checksum_file(filename_input, algo_kind, length_input, opts) {
            Err(UError(e)) => return Err(e),
            Err(Failed | CantOpenChecksumFile) => failed = true,
            Ok(_) => (),
        }
    }

    if failed {
        Err(USimpleError::new(1, ""))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn test_algo_based_parser() {
        #[allow(clippy::type_complexity)]
        let test_cases: &[(&[u8], Option<(&[u8], Option<&[u8]>, &[u8], &[u8])>)] = &[
            (b"SHA256 (example.txt) = d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2", Some((b"SHA256", None, b"example.txt", b"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2"))),
            // cspell:disable
            (b"BLAKE2b-512 (file) = abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef", Some((b"BLAKE2b", Some(b"512"), b"file", b"abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef"))),
            (b" MD5 (test) = 9e107d9d372bb6826bd81d3542a419d6", Some((b"MD5", None, b"test", b"9e107d9d372bb6826bd81d3542a419d6"))),
            (b"SHA-1 (anotherfile) = a9993e364706816aba3e25717850c26c9cd0d89d", Some((b"SHA", Some(b"1"), b"anotherfile", b"a9993e364706816aba3e25717850c26c9cd0d89d"))),
            (b" MD5 (anothertest) = fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"anothertest", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5(anothertest2) = fds65dsf46as5df4d6f54asds5d7f7g9", None),
            (b" MD5(weirdfilename0)= stillfilename)= fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"weirdfilename0)= stillfilename", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5(weirdfilename1)= )= fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"weirdfilename1)= ", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5(weirdfilename2) = )= fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"weirdfilename2) = ", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5 (weirdfilename3)= ) = fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"weirdfilename3)= ", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5 (weirdfilename4) = ) = fds65dsf46as5df4d6f54asds5d7f7g9", Some((b"MD5", None, b"weirdfilename4) = ", b"fds65dsf46as5df4d6f54asds5d7f7g9"))),
            (b" MD5(weirdfilename5)= ) = fds65dsf46as5df4d6f54asds5d7f7g9", None),
            (b" MD5(weirdfilename6) = ) = fds65dsf46as5df4d6f54asds5d7f7g9", None),
            (b" MD5 (weirdfilename7)= )= fds65dsf46as5df4d6f54asds5d7f7g9", None),
            (b" MD5 (weirdfilename8) = )= fds65dsf46as5df4d6f54asds5d7f7g9", None),
        ];

        // cspell:enable
        for (input, expected) in test_cases {
            let line_info = LineFormat::parse_algo_based(input);
            match expected {
                Some((algo, bits, filename, checksum)) => {
                    assert!(
                        line_info.is_some(),
                        "expected Some, got None for {}",
                        String::from_utf8_lossy(filename)
                    );
                    let line_info = line_info.unwrap();
                    assert_eq!(
                        &line_info.algo_name.unwrap().as_bytes(),
                        algo,
                        "failed for {}",
                        String::from_utf8_lossy(filename)
                    );
                    assert_eq!(
                        line_info
                            .algo_bit_len
                            .map(|m| m.to_string().as_bytes().to_owned()),
                        bits.map(|b| b.to_owned()),
                        "failed for {}",
                        String::from_utf8_lossy(filename)
                    );
                    assert_eq!(
                        &line_info.filename,
                        filename,
                        "failed for {}",
                        String::from_utf8_lossy(filename)
                    );
                    assert_eq!(
                        &line_info.checksum.as_bytes(),
                        checksum,
                        "failed for {}",
                        String::from_utf8_lossy(filename)
                    );
                }
                None => {
                    assert!(
                        line_info.is_none(),
                        "failed for {}",
                        String::from_utf8_lossy(input)
                    );
                }
            }
        }
    }

    #[test]
    fn test_double_space_parser() {
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
            // base64 checksums are accepted
            (
                b"b21lbGV0dGUgZHUgZnJvbWFnZQ==   ",
                Some((b"b21lbGV0dGUgZHUgZnJvbWFnZQ==", b" ")),
            ),
            // Invalid checksums fail
            (b"inva|idchecksum  test", None),
        ];

        for (input, expected) in test_cases {
            let line_info = LineFormat::parse_untagged(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(line_info.is_some());
                    let line_info = line_info.unwrap();
                    assert_eq!(&line_info.filename, filename);
                    assert_eq!(&line_info.checksum.as_bytes(), checksum);
                }
                None => {
                    assert!(line_info.is_none());
                }
            }
        }
    }

    #[test]
    fn test_single_space_parser() {
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
            let line_info = LineFormat::parse_single_space(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(line_info.is_some());
                    let line_info = line_info.unwrap();
                    assert_eq!(&line_info.filename, filename);
                    assert_eq!(&line_info.checksum.as_bytes(), checksum);
                }
                None => {
                    assert!(line_info.is_none());
                }
            }
        }
    }

    #[test]
    fn test_line_info() {
        let mut cached_line_format = None;

        // Test algo-based parser
        let line_algo_based =
            OsString::from("MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e");
        let line_info = LineInfo::parse(&line_algo_based, &mut cached_line_format).unwrap();
        assert_eq!(line_info.algo_name.as_deref(), Some("MD5"));
        assert!(line_info.algo_bit_len.is_none());
        assert_eq!(line_info.filename, b"example.txt");
        assert_eq!(line_info.checksum, "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(line_info.format, LineFormat::AlgoBased);
        assert!(cached_line_format.is_none());

        // Test double-space parser
        let line_double_space = OsString::from("d41d8cd98f00b204e9800998ecf8427e  example.txt");
        let line_info = LineInfo::parse(&line_double_space, &mut cached_line_format).unwrap();
        assert!(line_info.algo_name.is_none());
        assert!(line_info.algo_bit_len.is_none());
        assert_eq!(line_info.filename, b"example.txt");
        assert_eq!(line_info.checksum, "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(line_info.format, LineFormat::Untagged);
        assert!(cached_line_format.is_some());

        cached_line_format = None;

        // Test single-space parser
        let line_single_space = OsString::from("d41d8cd98f00b204e9800998ecf8427e example.txt");
        let line_info = LineInfo::parse(&line_single_space, &mut cached_line_format).unwrap();
        assert!(line_info.algo_name.is_none());
        assert!(line_info.algo_bit_len.is_none());
        assert_eq!(line_info.filename, b"example.txt");
        assert_eq!(line_info.checksum, "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(line_info.format, LineFormat::SingleSpace);
        assert!(cached_line_format.is_some());

        cached_line_format = None;

        // Test invalid checksum line
        let line_invalid = OsString::from("invalid checksum line");
        assert!(LineInfo::parse(&line_invalid, &mut cached_line_format).is_none());
        assert!(cached_line_format.is_none());

        // Test leading space before checksum line
        let line_algo_based_leading_space =
            OsString::from("   MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e");
        let line_info =
            LineInfo::parse(&line_algo_based_leading_space, &mut cached_line_format).unwrap();
        assert_eq!(line_info.format, LineFormat::AlgoBased);
        assert!(cached_line_format.is_none());

        // Test trailing space after checksum line (should fail)
        let line_algo_based_leading_space =
            OsString::from("MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e ");
        let res = LineInfo::parse(&line_algo_based_leading_space, &mut cached_line_format);
        assert!(res.is_none());
        assert!(cached_line_format.is_none());
    }

    #[test]
    fn test_get_expected_digest() {
        let ck = "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=".to_owned();

        let result = get_raw_expected_digest(&ck, None);

        assert_eq!(
            result.unwrap(),
            hex::decode(b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .unwrap()
        );
    }

    #[test]
    fn test_get_expected_checksum_invalid() {
        // The line misses a '=' at the end to be valid base64
        let ck = "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU".to_owned();

        let result = get_raw_expected_digest(&ck, None);

        assert!(result.is_none());
    }

    #[test]
    fn test_print_file_report() {
        let opts = ChecksumValidateOptions::default();

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
            print_file_report(&mut buffer, filename, *result, prefix, opts.verbose);
            assert_eq!(&buffer, expected);
        }
    }
}
