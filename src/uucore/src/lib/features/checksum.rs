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
    // Legacy aliases for -a sha2 -l xxx
    ALGORITHM_OPTIONS_SHA224,
    ALGORITHM_OPTIONS_SHA256,
    ALGORITHM_OPTIONS_SHA384,
    ALGORITHM_OPTIONS_SHA512,
    // Extra algorithms that are not valid `cksum --algorithm` as per GNU.
    // TODO: Should we keep them or drop them to align our support with GNU ?
    ALGORITHM_OPTIONS_BLAKE3,
    ALGORITHM_OPTIONS_SHAKE128,
    ALGORITHM_OPTIONS_SHAKE256,
];

/// Represents an algorithm kind. In some cases, it is not sufficient by itself
/// to know which algorithm to use exactly, because it lacks a digest length,
/// which is why [`SizedAlgoKind`] exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoKind {
    Sysv,
    Bsd,
    Crc,
    Crc32b,
    Md5,
    Sm3,
    Sha1,
    Sha2,
    Sha3,
    Blake2b,

    // Available in cksum for backward compatibility
    Sha224,
    Sha256,
    Sha384,
    Sha512,

    // Not available in cksum
    Shake128,
    Shake256,
    Blake3,
}

impl AlgoKind {
    /// Parses an [`AlgoKind`] from a string, only accepting valid cksum
    /// `--algorithm` values.
    pub fn from_cksum(algo: impl AsRef<str>) -> UResult<Self> {
        use AlgoKind::*;
        Ok(match algo.as_ref() {
            ALGORITHM_OPTIONS_SYSV => Sysv,
            ALGORITHM_OPTIONS_BSD => Bsd,
            ALGORITHM_OPTIONS_CRC => Crc,
            ALGORITHM_OPTIONS_CRC32B => Crc32b,
            ALGORITHM_OPTIONS_MD5 => Md5,
            ALGORITHM_OPTIONS_SHA1 => Sha1,
            ALGORITHM_OPTIONS_SHA2 => Sha2,
            ALGORITHM_OPTIONS_SHA3 => Sha3,
            ALGORITHM_OPTIONS_BLAKE2B => Blake2b,
            ALGORITHM_OPTIONS_SM3 => Sm3,

            // For backward compatibility
            ALGORITHM_OPTIONS_SHA224 => Sha224,
            ALGORITHM_OPTIONS_SHA256 => Sha256,
            ALGORITHM_OPTIONS_SHA384 => Sha384,
            ALGORITHM_OPTIONS_SHA512 => Sha512,
            _ => return Err(ChecksumError::UnknownAlgorithm(algo.as_ref().to_string()).into()),
        })
    }

    /// Parses an algo kind from a string, accepting standalone binary names.
    pub fn from_bin_name(algo: impl AsRef<str>) -> UResult<Self> {
        use AlgoKind::*;
        Ok(match algo.as_ref() {
            "md5sum" => Md5,
            "sha1sum" => Sha1,
            "sha224sum" => Sha224,
            "sha256sum" => Sha256,
            "sha384sum" => Sha384,
            "sha512sum" => Sha512,
            "sha3sum" => Sha3,
            "b2sum" => Blake2b,

            _ => return Err(ChecksumError::UnknownAlgorithm(algo.as_ref().to_string()).into()),
        })
    }

    /// Returns a string corresponding to the algorithm kind.
    pub fn to_uppercase(self) -> &'static str {
        use AlgoKind::*;
        match self {
            // Legacy algorithms
            Sysv => "SYSV",
            Bsd => "BSD",
            Crc => "CRC",
            Crc32b => "CRC32B",

            Md5 => "MD5",
            Sm3 => "SM3",
            Sha1 => "SHA1",
            Sha2 => "SHA2",
            Sha3 => "SHA3",
            Blake2b => "BLAKE2b", // Note the lowercase b in the end here.

            // For backward compatibility
            Sha224 => "SHA224",
            Sha256 => "SHA256",
            Sha384 => "SHA384",
            Sha512 => "SHA512",

            Shake128 => "SHAKE128",
            Shake256 => "SHAKE256",
            Blake3 => "BLAKE3",
        }
    }

    /// Returns a string corresponding to the algorithm option in cksum `-a`
    pub fn to_lowercase(self) -> &'static str {
        use AlgoKind::*;
        match self {
            Sysv => "sysv",
            Bsd => "bsd",
            Crc => "crc",
            Crc32b => "crc32b",
            Md5 => "md5",
            Sm3 => "sm3",
            Sha1 => "sha1",
            Sha2 => "sha2",
            Sha3 => "sha3",
            Blake2b => "blake2b",

            // For backward compatibility
            Sha224 => "sha224",
            Sha256 => "sha256",
            Sha384 => "sha384",
            Sha512 => "sha512",

            Shake128 => "shake128",
            Shake256 => "shake256",
            Blake3 => "blake3",
        }
    }

    pub fn is_legacy(self) -> bool {
        use AlgoKind::*;
        matches!(self, Sysv | Bsd | Crc | Crc32b)
    }
}

/// Holds a length for a SHA2 of SHA3 algorithm kind.
#[derive(Debug, Clone, Copy)]
pub enum ShaLength {
    Len224,
    Len256,
    Len384,
    Len512,
}

impl ShaLength {
    pub fn as_usize(self) -> usize {
        match self {
            Self::Len224 => 224,
            Self::Len256 => 256,
            Self::Len384 => 384,
            Self::Len512 => 512,
        }
    }
}

impl TryFrom<usize> for ShaLength {
    type Error = ChecksumError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        use ShaLength::*;
        match value {
            224 => Ok(Len224),
            256 => Ok(Len256),
            384 => Ok(Len384),
            512 => Ok(Len512),
            _ => Err(ChecksumError::InvalidLengthForSha(value.to_string())),
        }
    }
}

/// Represents an actual determined algorithm.
#[derive(Debug, Clone, Copy)]
pub enum SizedAlgoKind {
    Sysv,
    Bsd,
    Crc,
    Crc32b,
    Md5,
    Sm3,
    Sha1,
    Blake3,
    Sha2(ShaLength),
    Sha3(ShaLength),
    // Note: we store Blake2b's length as BYTES.
    Blake2b(Option<usize>),
    Shake128(usize),
    Shake256(usize),
}

impl SizedAlgoKind {
    pub fn from_unsized(kind: AlgoKind, byte_length: Option<usize>) -> UResult<Self> {
        use AlgoKind as ak;
        match (kind, byte_length) {
            (
                ak::Sysv
                | ak::Bsd
                | ak::Crc
                | ak::Crc32b
                | ak::Md5
                | ak::Sm3
                | ak::Sha1
                | ak::Blake3
                | ak::Sha224
                | ak::Sha256
                | ak::Sha384
                | ak::Sha512,
                Some(_),
            ) => Err(ChecksumError::LengthOnlyForBlake2bSha2Sha3.into()),

            (ak::Sysv, _) => Ok(Self::Sysv),
            (ak::Bsd, _) => Ok(Self::Bsd),
            (ak::Crc, _) => Ok(Self::Crc),
            (ak::Crc32b, _) => Ok(Self::Crc32b),
            (ak::Md5, _) => Ok(Self::Md5),
            (ak::Sm3, _) => Ok(Self::Sm3),
            (ak::Sha1, _) => Ok(Self::Sha1),
            (ak::Blake3, _) => Ok(Self::Blake3),

            (ak::Shake128, Some(l)) => Ok(Self::Shake128(l)),
            (ak::Shake256, Some(l)) => Ok(Self::Shake256(l)),
            (ak::Sha2, Some(l)) => Ok(Self::Sha2(ShaLength::try_from(l)?)),
            (ak::Sha3, Some(l)) => Ok(Self::Sha3(ShaLength::try_from(l)?)),
            (algo @ (ak::Sha2 | ak::Sha3), None) => {
                Err(ChecksumError::LengthRequiredForSha(algo.to_lowercase().into()).into())
            }
            // [`calculate_blake2b_length`] expects a length in bits but we
            // have a length in bytes.
            (ak::Blake2b, Some(l)) => Ok(Self::Blake2b(calculate_blake2b_length(8 * l)?)),
            (ak::Blake2b, None) => Ok(Self::Blake2b(None)),

            (ak::Sha224, None) => Ok(Self::Sha2(ShaLength::Len224)),
            (ak::Sha256, None) => Ok(Self::Sha2(ShaLength::Len256)),
            (ak::Sha384, None) => Ok(Self::Sha2(ShaLength::Len384)),
            (ak::Sha512, None) => Ok(Self::Sha2(ShaLength::Len512)),
            (_, None) => Err(ChecksumError::LengthRequired(kind.to_uppercase().into()).into()),
        }
    }

    pub fn to_tag(self) -> String {
        use SizedAlgoKind::*;
        match self {
            Md5 => "MD5".into(),
            Sm3 => "SM3".into(),
            Sha1 => "SHA1".into(),
            Blake3 => "BLAKE3".into(),
            Sha2(len) => format!("SHA{}", len.as_usize()),
            Sha3(len) => format!("SHA3-{}", len.as_usize()),
            Blake2b(Some(byte_len)) => format!("BLAKE2b-{}", byte_len * 8),
            Blake2b(None) => "BLAKE2b".into(),
            Shake128(_) => "SHAKE128".into(),
            Shake256(_) => "SHAKE256".into(),
            Sysv | Bsd | Crc | Crc32b => panic!("Should not be used for tagging"),
        }
    }

    pub fn create_digest(&self) -> Box<dyn Digest + 'static> {
        use ShaLength::*;
        match self {
            Self::Sysv => Box::new(SysV::new()),
            Self::Bsd => Box::new(Bsd::new()),
            Self::Crc => Box::new(Crc::new()),
            Self::Crc32b => Box::new(CRC32B::new()),
            Self::Md5 => Box::new(Md5::new()),
            Self::Sm3 => Box::new(Sm3::new()),
            Self::Sha1 => Box::new(Sha1::new()),
            Self::Blake3 => Box::new(Blake3::new()),
            Self::Sha2(Len224) => Box::new(Sha224::new()),
            Self::Sha2(Len256) => Box::new(Sha256::new()),
            Self::Sha2(Len384) => Box::new(Sha384::new()),
            Self::Sha2(Len512) => Box::new(Sha512::new()),
            Self::Sha3(Len224) => Box::new(Sha3_224::new()),
            Self::Sha3(Len256) => Box::new(Sha3_256::new()),
            Self::Sha3(Len384) => Box::new(Sha3_384::new()),
            Self::Sha3(Len512) => Box::new(Sha3_512::new()),
            Self::Blake2b(Some(byte_len)) => Box::new(Blake2b::with_output_bytes(*byte_len)),
            Self::Blake2b(None) => Box::new(Blake2b::new()),
            Self::Shake128(_) => Box::new(Shake128::new()),
            Self::Shake256(_) => Box::new(Shake256::new()),
        }
    }

    pub fn bitlen(&self) -> usize {
        use SizedAlgoKind::*;
        match self {
            Sysv => 512,
            Bsd => 1024,
            Crc => 256,
            Crc32b => 32,
            Md5 => 128,
            Sm3 => 512,
            Sha1 => 160,
            Blake3 => 256,
            Sha2(len) => len.as_usize(),
            Sha3(len) => len.as_usize(),
            Blake2b(len) => len.unwrap_or(512),
            Shake128(len) => *len,
            Shake256(len) => *len,
        }
    }
    pub fn is_legacy(&self) -> bool {
        use SizedAlgoKind::*;
        matches!(self, Sysv | Bsd | Crc | Crc32b)
    }
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

    // --length sanitization errors
    #[error("--length required for {}", .0.quote())]
    LengthRequired(String),
    #[error("invalid length: {}", .0.quote())]
    InvalidLength(String),
    #[error("maximum digest length for {} is 512 bits", .0.quote())]
    LengthTooBigForBlake(String),
    #[error("length is not a multiple of 8")]
    LengthNotMultipleOf8,
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
    expected_checksum: &str,
    algo: SizedAlgoKind,
    opts: ChecksumOptions,
) -> Result<(), LineCheckError> {
    let (filename_to_check_unescaped, prefix) = unescape_filename(filename);
    let real_filename_to_check = os_str_from_bytes(&filename_to_check_unescaped)?;

    // Open the input file
    let file_to_check = get_file_to_check(&real_filename_to_check, opts)?;
    let mut file_reader = BufReader::new(file_to_check);

    // Read the file and calculate the checksum
    let mut digest = algo.create_digest();
    let (calculated_checksum, _) =
        digest_reader(&mut digest, &mut file_reader, opts.binary, algo.bitlen()).unwrap();

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
    cli_algo_kind: Option<AlgoKind>,
    opts: ChecksumOptions,
    last_algo: &mut Option<String>,
) -> Result<(), LineCheckError> {
    let filename_to_check = line_info.filename.as_slice();

    let (algo_kind, algo_byte_len) =
        identify_algo_name_and_length(line_info, cli_algo_kind, last_algo)?;

    // If the digest bitlen is known, we can check the format of the expected
    // checksum with it.
    let digest_char_length_hint = match (algo_kind, algo_byte_len) {
        (AlgoKind::Blake2b, Some(bytelen)) => Some(bytelen * 2),
        _ => None,
    };

    let expected_checksum = get_expected_digest_as_hex_string(line_info, digest_char_length_hint)
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
    let (algo_kind, algo_byte_len) = match cli_algo_kind {
        AlgoKind::Blake2b => {
            // division by 2 converts the length of the Blake2b checksum from
            // hexadecimal characters to bytes, as each byte is represented by
            // two hexadecimal characters.
            (AlgoKind::Blake2b, Some(expected_checksum.len() / 2))
        }
        algo @ (AlgoKind::Sha2 | AlgoKind::Sha3) => {
            // multiplication by 4 to get the number of bits
            (algo, Some(expected_checksum.len() * 4))
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
    cli_algo_kind: Option<AlgoKind>,
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
    algo_kind: Option<AlgoKind>,
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
pub fn calculate_blake2b_length(bit_length: usize) -> UResult<Option<usize>> {
    calculate_blake2b_length_str(bit_length.to_string().as_str())
}

/// Calculates the length of the digest.
pub fn calculate_blake2b_length_str(bit_length: &str) -> UResult<Option<usize>> {
    // Blake2b's length is parsed in an u64.
    match bit_length.parse::<usize>() {
        Ok(0) => Ok(None),

        // Error cases
        Ok(n) if n > 512 => {
            show_error!("{}", ChecksumError::InvalidLength(bit_length.into()));
            Err(ChecksumError::LengthTooBigForBlake("BLAKE2b".into()).into())
        }
        Err(e) if *e.kind() == IntErrorKind::PosOverflow => {
            show_error!("{}", ChecksumError::InvalidLength(bit_length.into()));
            Err(ChecksumError::LengthTooBigForBlake("BLAKE2b".into()).into())
        }
        Err(_) => Err(ChecksumError::InvalidLength(bit_length.into()).into()),

        Ok(n) if n % 8 != 0 => {
            show_error!("{}", ChecksumError::InvalidLength(bit_length.into()));
            Err(ChecksumError::LengthNotMultipleOf8.into())
        }

        // Valid cases

        // When length is 512, it is blake2b's default. So, don't show it
        Ok(512) => Ok(None),
        // Divide by 8, as our blake2b implementation expects bytes instead of bits.
        Ok(n) => Ok(Some(n / 8)),
    }
}

pub fn validate_sha2_sha3_length(algo_name: AlgoKind, length: Option<usize>) -> UResult<ShaLength> {
    match length {
        Some(224) => Ok(ShaLength::Len224),
        Some(256) => Ok(ShaLength::Len256),
        Some(384) => Ok(ShaLength::Len384),
        Some(512) => Ok(ShaLength::Len512),
        Some(len) => {
            show_error!("{}", ChecksumError::InvalidLength(len.to_string()));
            Err(ChecksumError::InvalidLengthForSha(algo_name.to_uppercase().into()).into())
        }
        None => Err(ChecksumError::LengthRequiredForSha(algo_name.to_lowercase().into()).into()),
    }
}

pub fn sanitize_sha2_sha3_length_str(algo_kind: AlgoKind, length: &str) -> UResult<usize> {
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
            return Err(ChecksumError::InvalidLengthForSha(algo_kind.to_uppercase().into()).into());
        }
        Err(_) => return Err(ChecksumError::InvalidLength(length.into()).into()),
    };

    if [224, 256, 384, 512].contains(&len) {
        Ok(len)
    } else {
        show_error!("{}", ChecksumError::InvalidLength(length.into()));
        Err(ChecksumError::InvalidLengthForSha(algo_kind.to_uppercase().into()).into())
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
