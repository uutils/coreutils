// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore anotherfile invalidchecksum JWZG FFFD xffname prefixfilename bytelen bitlen hexdigit rsplit

use data_encoding::BASE64;
use os_display::Quotable;
use std::{
    borrow::Cow,
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::{self, BufReader, Read, Write, stdin},
    num::IntErrorKind,
    path::Path,
    str,
};

use crate::{
    error::{FromIo, UError, UResult, USimpleError},
    os_str_as_bytes, os_str_from_bytes,
    quoting_style::{QuotingStyle, locale_aware_escape_name},
    read_os_string_lines, show, show_error, show_warning_caps,
    sum::{
        Blake2b, Blake3, Bsd, CRC32B, Crc, Digest, DigestWriter, Md5, Sha1, Sha3_224, Sha3_256,
        Sha3_384, Sha3_512, Sha224, Sha256, Sha384, Sha512, Shake128, Shake256, Sm3, SysV,
    },
    util_name,
};
use thiserror::Error;

pub const ALGORITHM_OPTIONS_SYSV: &str = "sysv";
pub const ALGORITHM_OPTIONS_BSD: &str = "bsd";
pub const ALGORITHM_OPTIONS_CRC: &str = "crc";
pub const ALGORITHM_OPTIONS_CRC32B: &str = "crc32b";
pub const ALGORITHM_OPTIONS_MD5: &str = "md5";
pub const ALGORITHM_OPTIONS_SHA1: &str = "sha1";
pub const ALGORITHM_OPTIONS_SHA2: &str = "sha2";
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

pub const SUPPORTED_ALGORITHMS: [&str; 17] = [
    ALGORITHM_OPTIONS_SYSV,
    ALGORITHM_OPTIONS_BSD,
    ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_CRC32B,
    ALGORITHM_OPTIONS_MD5,
    ALGORITHM_OPTIONS_SHA1,
    ALGORITHM_OPTIONS_SHA2,
    ALGORITHM_OPTIONS_SHA3,
    ALGORITHM_OPTIONS_BLAKE2B,
    ALGORITHM_OPTIONS_SM3,
    // Extra algorithms that are not valid `cksum --algorithm`
    ALGORITHM_OPTIONS_SHA224,
    ALGORITHM_OPTIONS_SHA256,
    ALGORITHM_OPTIONS_SHA384,
    ALGORITHM_OPTIONS_SHA512,
    ALGORITHM_OPTIONS_BLAKE3,
    ALGORITHM_OPTIONS_SHAKE128,
    ALGORITHM_OPTIONS_SHAKE256,
];

pub const LEGACY_ALGORITHMS: [&str; 4] = [
    ALGORITHM_OPTIONS_SYSV,
    ALGORITHM_OPTIONS_BSD,
    ALGORITHM_OPTIONS_CRC,
    ALGORITHM_OPTIONS_CRC32B,
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
pub struct ChecksumOptions {
    pub binary: bool,
    pub ignore_missing: bool,
    pub strict: bool,
    pub verbose: ChecksumVerbose,
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
    #[error("--length required for {}", .0.quote())]
    LengthRequired(String),
    #[error("invalid length: {}", .0.quote())]
    InvalidLength(String),
    #[error("digest length for {} must be 224, 256, 384, or 512", .0.quote())]
    InvalidLengthForSha(String),
    #[error("--algorithm={0} requires specifying --length 224, 256, 384, or 512")]
    LengthRequiredForSha(String),
    #[error("--length is only supported with --algorithm blake2b, sha2, or sha3")]
    LengthOnlyForBlake2bSha2Sha3,
    #[error("the --binary and --text options are meaningless when verifying checksums")]
    BinaryTextConflict,
    #[error("--text mode is only supported with --untagged")]
    TextWithoutUntagged,
    #[error("--check is not supported with --algorithm={{bsd,sysv,crc,crc32b}}")]
    AlgorithmNotSupportedWithCheck,
    #[error("You cannot combine multiple hash algorithms!")]
    CombineMultipleAlgorithms,
    #[error("Needs an algorithm to hash with.\nUse --help for more information.")]
    NeedAlgorithmToHash,
    #[error("unknown algorithm: {0}: clap should have prevented this case")]
    UnknownAlgorithm(String),
    #[error("")]
    Io(#[from] io::Error),
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
/// Returns a `UResult` with an `HashAlgorithm` or an `Err` if an unsupported
/// output size is provided.
pub fn create_sha3(bits: usize) -> UResult<HashAlgorithm> {
    match bits {
        224 => Ok(HashAlgorithm {
            name: "SHA3-224",
            create_fn: Box::new(|| Box::new(Sha3_224::new())),
            bits: 224,
        }),
        256 => Ok(HashAlgorithm {
            name: "SHA3-256",
            create_fn: Box::new(|| Box::new(Sha3_256::new())),
            bits: 256,
        }),
        384 => Ok(HashAlgorithm {
            name: "SHA3-384",
            create_fn: Box::new(|| Box::new(Sha3_384::new())),
            bits: 384,
        }),
        512 => Ok(HashAlgorithm {
            name: "SHA3-512",
            create_fn: Box::new(|| Box::new(Sha3_512::new())),
            bits: 512,
        }),

        _ => Err(ChecksumError::InvalidLengthForSha("SHA3".into()).into()),
    }
}

pub fn create_sha2(bits: usize) -> UResult<HashAlgorithm> {
    match bits {
        224 => Ok(HashAlgorithm {
            name: "SHA224",
            create_fn: Box::new(|| Box::new(Sha224::new())),
            bits: 224,
        }),
        256 => Ok(HashAlgorithm {
            name: "SHA256",
            create_fn: Box::new(|| Box::new(Sha256::new())),
            bits: 256,
        }),
        384 => Ok(HashAlgorithm {
            name: "SHA384",
            create_fn: Box::new(|| Box::new(Sha384::new())),
            bits: 384,
        }),
        512 => Ok(HashAlgorithm {
            name: "SHA512",
            create_fn: Box::new(|| Box::new(Sha512::new())),
            bits: 512,
        }),

        _ => Err(ChecksumError::InvalidLengthForSha("SHA2".into()).into()),
    }
}

#[allow(clippy::comparison_chain)]
fn print_cksum_report(res: &ChecksumResult) {
    if res.bad_format == 1 {
        show_warning_caps!("{} line is improperly formatted", res.bad_format);
    } else if res.bad_format > 1 {
        show_warning_caps!("{} lines are improperly formatted", res.bad_format);
    }

    if res.failed_cksum == 1 {
        show_warning_caps!("{} computed checksum did NOT match", res.failed_cksum);
    } else if res.failed_cksum > 1 {
        show_warning_caps!("{} computed checksums did NOT match", res.failed_cksum);
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

pub fn detect_algo(algo: &str, length: Option<usize>) -> UResult<HashAlgorithm> {
    match algo {
        ALGORITHM_OPTIONS_SYSV => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_SYSV,
            create_fn: Box::new(|| Box::new(SysV::new())),
            bits: 512,
        }),
        ALGORITHM_OPTIONS_BSD => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_BSD,
            create_fn: Box::new(|| Box::new(Bsd::new())),
            bits: 1024,
        }),
        ALGORITHM_OPTIONS_CRC => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_CRC,
            create_fn: Box::new(|| Box::new(Crc::new())),
            bits: 256,
        }),
        ALGORITHM_OPTIONS_CRC32B => Ok(HashAlgorithm {
            name: ALGORITHM_OPTIONS_CRC32B,
            create_fn: Box::new(|| Box::new(CRC32B::new())),
            bits: 32,
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
        ALGORITHM_OPTIONS_SHA224 | "sha224sum" => Ok(create_sha2(224)?),
        ALGORITHM_OPTIONS_SHA256 | "sha256sum" => Ok(create_sha2(256)?),
        ALGORITHM_OPTIONS_SHA384 | "sha384sum" => Ok(create_sha2(384)?),
        ALGORITHM_OPTIONS_SHA512 | "sha512sum" => Ok(create_sha2(512)?),
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
        algo @ (ALGORITHM_OPTIONS_SHAKE128 | "shake128sum") => {
            let bits = length.ok_or(ChecksumError::LengthRequired(algo.to_ascii_uppercase()))?;
            Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHAKE128,
                create_fn: Box::new(|| Box::new(Shake128::new())),
                bits,
            })
        }
        algo @ (ALGORITHM_OPTIONS_SHAKE256 | "shake256sum") => {
            let bits = length.ok_or(ChecksumError::LengthRequired(algo.to_ascii_uppercase()))?;
            Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHAKE256,
                create_fn: Box::new(|| Box::new(Shake256::new())),
                bits,
            })
        }
        algo @ ALGORITHM_OPTIONS_SHA2 => {
            let bits = validate_sha2_sha3_length(algo, length)?;
            create_sha2(bits)
        }
        algo @ ALGORITHM_OPTIONS_SHA3 => {
            let bits = validate_sha2_sha3_length(algo, length)?;
            create_sha3(bits)
        }

        // TODO: `hashsum` specific, to remove once hashsum is removed.
        algo @ ("sha3-224" | "sha3-256" | "sha3-384" | "sha3-512") => {
            let bits: usize = algo.strip_prefix("sha3-").unwrap().parse().unwrap();
            create_sha3(bits)
        }

        algo => Err(ChecksumError::UnknownAlgorithm(algo.into()).into()),
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

        fn is_valid_checksum(checksum: &[u8]) -> bool {
            if checksum.is_empty() {
                return false;
            }

            let mut parts = checksum.splitn(2, |&b| b == b'=');
            let main = parts.next().unwrap(); // Always exists since checksum isn't empty
            let padding = parts.next().unwrap_or_default(); // Empty if no '='

            main.iter()
                .all(|&b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
                && !main.is_empty()
                && padding.len() <= 2
                && padding.iter().all(|&b| b == b'=')
        }
        if !is_valid_checksum(checksum) {
            return None;
        }
        // SAFETY: we just validated the contents of checksum, we can unsafely make a
        // String from it
        let checksum_utf8 = unsafe { String::from_utf8_unchecked(checksum.to_vec()) };

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
        if !checksum.iter().all(|&b| b.is_ascii_hexdigit()) || checksum.is_empty() {
            return None;
        }
        // SAFETY: we just validated the contents of checksum, we can unsafely make a
        // String from it
        let checksum_utf8 = unsafe { String::from_utf8_unchecked(checksum.to_vec()) };

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

fn get_filename_for_output(filename: &OsStr, input_is_stdin: bool) -> String {
    if input_is_stdin {
        "standard input"
    } else {
        filename.to_str().unwrap()
    }
    .maybe_quote()
    .to_string()
}

/// Extract the expected digest from the checksum string
fn get_expected_digest_as_hex_string(
    line_info: &LineInfo,
    len_hint: Option<usize>,
) -> Option<Cow<'_, str>> {
    let ck = &line_info.checksum;

    let against_hint = |len| len_hint.is_none_or(|l| l == len);

    if ck.len() % 2 != 0 {
        // If the length of the digest is not a multiple of 2, then it
        // must be improperly formatted (1 hex digit is 2 characters)
        return None;
    }

    // If the digest can be decoded as hexadecimal AND its length matches the
    // one expected (in case it's given), just go with it.
    if ck.as_bytes().iter().all(u8::is_ascii_hexdigit) && against_hint(ck.len()) {
        return Some(Cow::Borrowed(ck));
    }

    // If hexadecimal digest fails for any reason, interpret the digest as base 64.
    BASE64
        .decode(ck.as_bytes()) // Decode the string as encoded base64
        .map(hex::encode) // Encode it back as hexadecimal
        .map(Cow::<str>::Owned)
        .ok()
        .and_then(|s| {
            // Check the digest length
            if against_hint(s.len()) { Some(s) } else { None }
        })
}

/// Returns a reader that reads from the specified file, or from stdin if `filename_to_check` is "-".
fn get_file_to_check(
    filename: &OsStr,
    opts: ChecksumOptions,
) -> Result<Box<dyn Read>, LineCheckError> {
    let filename_bytes = os_str_as_bytes(filename).expect("UTF-8 error");

    if filename == "-" {
        Ok(Box::new(stdin())) // Use stdin if "-" is specified in the checksum file
    } else {
        let failed_open = || {
            print_file_report(
                std::io::stdout(),
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
                Err(
                    io::Error::other(format!("{}: Is a directory", filename.to_string_lossy()))
                        .into(),
                )
            } else {
                Ok(Box::new(f))
            }
        }
        Err(_) => Err(io::Error::other(format!(
            "{}: No such file or directory",
            filename.to_string_lossy()
        ))
        .into()),
    }
}

/// Gets the algorithm name and length from the `LineInfo` if the algo-based format is matched.
fn identify_algo_name_and_length(
    line_info: &LineInfo,
    algo_name_input: Option<&str>,
    last_algo: &mut Option<String>,
) -> Result<(String, Option<usize>), LineCheckError> {
    let algo_from_line = line_info.algo_name.clone().unwrap_or_default();
    let line_algo = algo_from_line.to_lowercase();
    *last_algo = Some(algo_from_line);

    // check if we are called with XXXsum (example: md5sum) but we detected a
    // different algo parsing the file (for example SHA1 (f) = d...)
    //
    // Also handle the case cksum -s sm3 but the file contains other formats
    if let Some(algo_name_input) = algo_name_input {
        match (algo_name_input, line_algo.as_str()) {
            (l, r) if l == r => (),
            // Edge case for SHA2, which matches SHA(224|256|384|512)
            (
                ALGORITHM_OPTIONS_SHA2,
                ALGORITHM_OPTIONS_SHA224
                | ALGORITHM_OPTIONS_SHA256
                | ALGORITHM_OPTIONS_SHA384
                | ALGORITHM_OPTIONS_SHA512,
            ) => (),
            _ => return Err(LineCheckError::ImproperlyFormatted),
        }
    }

    if !SUPPORTED_ALGORITHMS.contains(&line_algo.as_str()) {
        // Not supported algo, leave early
        return Err(LineCheckError::ImproperlyFormatted);
    }

    let bytes = if let Some(bitlen) = line_info.algo_bit_len {
        match line_algo.as_str() {
            ALGORITHM_OPTIONS_BLAKE2B if bitlen % 8 == 0 => Some(bitlen / 8),
            ALGORITHM_OPTIONS_SHA2 | ALGORITHM_OPTIONS_SHA3
                if [224, 256, 384, 512].contains(&bitlen) =>
            {
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
    } else if line_algo == ALGORITHM_OPTIONS_BLAKE2B {
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
    expected_checksum: &str,
    mut algo: HashAlgorithm,
    opts: ChecksumOptions,
) -> Result<(), LineCheckError> {
    let (filename_to_check_unescaped, prefix) = unescape_filename(filename);
    let real_filename_to_check = os_str_from_bytes(&filename_to_check_unescaped)?;

    // Open the input file
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
    cli_algo_name: Option<&str>,
    opts: ChecksumOptions,
    last_algo: &mut Option<String>,
) -> Result<(), LineCheckError> {
    let filename_to_check = line_info.filename.as_slice();

    let (algo_name, algo_byte_len) =
        identify_algo_name_and_length(line_info, cli_algo_name, last_algo)?;

    // If the digest bitlen is known, we can check the format of the expected
    // checksum with it.
    let digest_char_length_hint = match (algo_name.as_str(), algo_byte_len) {
        (ALGORITHM_OPTIONS_BLAKE2B, Some(bytelen)) => Some(bytelen * 2),
        _ => None,
    };

    let expected_checksum = get_expected_digest_as_hex_string(line_info, digest_char_length_hint)
        .ok_or(LineCheckError::ImproperlyFormatted)?;

    let algo = detect_algo(&algo_name, algo_byte_len)?;

    compute_and_check_digest_from_file(filename_to_check, &expected_checksum, algo, opts)
}

/// Check a digest checksum with non-algo based pre-treatment.
fn process_non_algo_based_line(
    line_number: usize,
    line_info: &LineInfo,
    cli_algo_name: &str,
    cli_algo_length: Option<usize>,
    opts: ChecksumOptions,
) -> Result<(), LineCheckError> {
    let mut filename_to_check = line_info.filename.as_slice();
    if filename_to_check.starts_with(b"*")
        && line_number == 0
        && line_info.format == LineFormat::SingleSpace
    {
        // Remove the leading asterisk if present - only for the first line
        filename_to_check = &filename_to_check[1..];
    }
    let expected_checksum = get_expected_digest_as_hex_string(line_info, None)
        .ok_or(LineCheckError::ImproperlyFormatted)?;

    // When a specific algorithm name is input, use it and use the provided
    // bits except when dealing with blake2b, sha2 and sha3, where we will
    // detect the length.
    let (algo_name, algo_byte_len) = match cli_algo_name {
        ALGORITHM_OPTIONS_BLAKE2B => {
            // division by 2 converts the length of the Blake2b checksum from
            // hexadecimal characters to bytes, as each byte is represented by
            // two hexadecimal characters.
            (
                ALGORITHM_OPTIONS_BLAKE2B.to_string(),
                Some(expected_checksum.len() / 2),
            )
        }
        algo @ (ALGORITHM_OPTIONS_SHA2 | ALGORITHM_OPTIONS_SHA3) => {
            // multiplication by 4 to get the number of bits
            (algo.to_string(), Some(expected_checksum.len() * 4))
        }
        _ => (cli_algo_name.to_lowercase(), cli_algo_length),
    };

    let algo = detect_algo(&algo_name, algo_byte_len)?;

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
    cli_algo_name: Option<&str>,
    cli_algo_length: Option<usize>,
    opts: ChecksumOptions,
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
            cli_algo_name,
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
                    let algo = if let Some(algo_name_input) = cli_algo_name {
                        Cow::Owned(algo_name_input.to_uppercase())
                    } else if let Some(algo) = &last_algo {
                        Cow::Borrowed(algo.as_str())
                    } else {
                        Cow::Borrowed("Unknown algorithm")
                    };
                    eprintln!(
                        "{}: {}: {}: improperly formatted {algo} checksum line",
                        util_name(),
                        filename_input.maybe_quote(),
                        i + 1,
                    );
                }
            }
            Err(CantOpenFile | FileIsDirectory) => res.failed_open_file += 1,
            Err(FileNotFound) if !opts.ignore_missing => res.failed_open_file += 1,
            _ => (),
        }
    }

    // not a single line correctly formatted found
    // return an error
    if res.total_properly_formatted() == 0 {
        if opts.verbose.over_status() {
            log_no_properly_formatted(get_filename_for_output(filename_input, input_is_stdin));
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
            eprintln!(
                "{}: {}: no file was verified",
                util_name(),
                filename_input.maybe_quote(),
            );
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
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
    opts: ChecksumOptions,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut failed = false;

    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        use FileCheckError::*;
        match process_checksum_file(filename_input, algo_name_input, length_input, opts) {
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

pub fn digest_reader<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut T,
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
        let mut bytes = vec![0; output_bits.div_ceil(8)];
        digest.hash_finalize(&mut bytes);
        Ok((hex::encode(bytes), output_size))
    }
}

/// Calculates the length of the digest.
pub fn calculate_blake2b_length(length: usize) -> UResult<Option<usize>> {
    calculate_blake2b_length_str(length.to_string().as_str())
}

/// Calculates the length of the digest.
pub fn calculate_blake2b_length_str(length: &str) -> UResult<Option<usize>> {
    match length.parse() {
        Ok(0) => Ok(None),
        Ok(n) if n % 8 != 0 => {
            show_error!("{}", ChecksumError::InvalidLength(length.into()));
            Err(io::Error::new(io::ErrorKind::InvalidInput, "length is not a multiple of 8").into())
        }
        Ok(n) if n > 512 => {
            show_error!("{}", ChecksumError::InvalidLength(length.into()));
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "maximum digest length for {} is 512 bits",
                    "BLAKE2b".quote()
                ),
            )
            .into())
        }
        Ok(n) => {
            // Divide by 8, as our blake2b implementation expects bytes instead of bits.
            if n == 512 {
                // When length is 512, it is blake2b's default.
                // So, don't show it
                Ok(None)
            } else {
                Ok(Some(n / 8))
            }
        }
        Err(_) => Err(ChecksumError::InvalidLength(length.into()).into()),
    }
}

pub fn validate_sha2_sha3_length(algo_name: &str, length: Option<usize>) -> UResult<usize> {
    match length {
        Some(len @ (224 | 256 | 384 | 512)) => Ok(len),
        Some(len) => {
            show_error!("{}", ChecksumError::InvalidLength(len.to_string()));
            Err(ChecksumError::InvalidLengthForSha(algo_name.to_ascii_uppercase()).into())
        }
        None => Err(ChecksumError::LengthRequiredForSha(algo_name.into()).into()),
    }
}

pub fn sanitize_sha2_sha3_length_str(algo_name: &str, length: &str) -> UResult<usize> {
    // There is a difference in the errors sent when the length is not a number
    // vs. its an invalid number.
    //
    // When inputting an invalid number, an extra error message it printed to
    // remind of the accepted inputs.
    let len = match length.parse::<usize>() {
        Ok(l) => l,
        // Note: Positive overflow while parsing counts as an invalid number,
        // but a number still.
        Err(e) if *e.kind() == IntErrorKind::PosOverflow => {
            show_error!("{}", ChecksumError::InvalidLength(length.into()));
            return Err(ChecksumError::InvalidLengthForSha(algo_name.to_ascii_uppercase()).into());
        }
        Err(_) => return Err(ChecksumError::InvalidLength(length.into()).into()),
    };

    if [224, 256, 384, 512].contains(&len) {
        Ok(len)
    } else {
        show_error!("{}", ChecksumError::InvalidLength(length.into()));
        Err(ChecksumError::InvalidLengthForSha(algo_name.to_ascii_uppercase()).into())
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
            ALGORITHM_OPTIONS_SHA224.to_ascii_uppercase()
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA256, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA256.to_ascii_uppercase()
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA384, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA384.to_ascii_uppercase()
        );
        assert_eq!(
            detect_algo(ALGORITHM_OPTIONS_SHA512, None).unwrap().name,
            ALGORITHM_OPTIONS_SHA512.to_ascii_uppercase()
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

        // Older versions of checksum used to detect the "sha3" prefix, but not
        // anymore.
        assert!(detect_algo("sha3_224", Some(224)).is_err());
        assert!(detect_algo("sha3_256", Some(256)).is_err());
        assert!(detect_algo("sha3_384", Some(384)).is_err());
        assert!(detect_algo("sha3_512", Some(512)).is_err());

        let sha3_224 = detect_algo("sha3", Some(224)).unwrap();
        assert_eq!(sha3_224.name, "SHA3-224");
        assert_eq!(sha3_224.bits, 224);
        let sha3_256 = detect_algo("sha3", Some(256)).unwrap();
        assert_eq!(sha3_256.name, "SHA3-256");
        assert_eq!(sha3_256.bits, 256);
        let sha3_384 = detect_algo("sha3", Some(384)).unwrap();
        assert_eq!(sha3_384.name, "SHA3-384");
        assert_eq!(sha3_384.bits, 384);
        let sha3_512 = detect_algo("sha3", Some(512)).unwrap();
        assert_eq!(sha3_512.name, "SHA3-512");
        assert_eq!(sha3_512.bits, 512);

        assert!(detect_algo("sha3", None).is_err());

        assert_eq!(detect_algo("sha2", Some(224)).unwrap().name, "SHA224");
        assert_eq!(detect_algo("sha2", Some(256)).unwrap().name, "SHA256");
        assert_eq!(detect_algo("sha2", Some(384)).unwrap().name, "SHA384");
        assert_eq!(detect_algo("sha2", Some(512)).unwrap().name, "SHA512");

        assert!(detect_algo("sha2", None).is_err());
    }

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
            (b"invalidchecksum  test", None),
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
        let line = OsString::from("SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=");
        let mut cached_line_format = None;
        let line_info = LineInfo::parse(&line, &mut cached_line_format).unwrap();

        let result = get_expected_digest_as_hex_string(&line_info, None);

        assert_eq!(
            result.unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_get_expected_checksum_invalid() {
        // The line misses a '=' at the end to be valid base64
        let line = OsString::from("SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU");
        let mut cached_line_format = None;
        let line_info = LineInfo::parse(&line, &mut cached_line_format).unwrap();

        let result = get_expected_digest_as_hex_string(&line_info, None);

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
            print_file_report(&mut buffer, filename, *result, prefix, opts.verbose);
            assert_eq!(&buffer, expected);
        }
    }
}
