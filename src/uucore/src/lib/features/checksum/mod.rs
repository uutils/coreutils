// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bitlen

use std::ffi::OsStr;
use std::io::{self, Read};
use std::num::IntErrorKind;

use os_display::Quotable;
use thiserror::Error;

use crate::error::{UError, UResult};
use crate::show_error;
use crate::sum::{
    Blake2b, Blake3, Bsd, CRC32B, Crc, Digest, DigestOutput, DigestWriter, Md5, Sha1, Sha3_224,
    Sha3_256, Sha3_384, Sha3_512, Sha224, Sha256, Sha384, Sha512, Shake128, Shake256, Sm3, SysV,
};

pub mod compute;
pub mod validate;

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
            (ak::Blake2b, Some(l)) => Ok(Self::Blake2b(calculate_blake2b_length_str(
                &(8 * l).to_string(),
            )?)),
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

#[derive(Debug, Error)]
pub enum ChecksumError {
    #[error("the --raw option is not supported with multiple files")]
    RawMultipleFiles,

    #[error("the --{0} option is meaningful only when verifying checksums")]
    CheckOnlyFlag(String),

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

pub fn digest_reader<T: Read>(
    digest: &mut Box<dyn Digest>,
    reader: &mut T,
    binary: bool,
) -> io::Result<(DigestOutput, usize)> {
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

    Ok((digest.result(), output_size))
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

pub fn escape_filename(filename: &OsStr) -> (String, &'static str) {
    let original = filename.to_string_lossy();
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
        let (escaped, prefix) = escape_filename(OsStr::new("testfile.txt"));
        assert_eq!(escaped, "testfile.txt");
        assert_eq!(prefix, "");

        let (escaped, prefix) = escape_filename(OsStr::new("test\nfile.txt"));
        assert_eq!(escaped, "test\\nfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(OsStr::new("test\rfile.txt"));
        assert_eq!(escaped, "test\\rfile.txt");
        assert_eq!(prefix, "\\");

        let (escaped, prefix) = escape_filename(OsStr::new("test\\file.txt"));
        assert_eq!(escaped, "test\\\\file.txt");
        assert_eq!(prefix, "\\");
    }

    #[test]
    fn test_calculate_blake2b_length() {
        assert_eq!(calculate_blake2b_length_str("0").unwrap(), None);
        assert!(calculate_blake2b_length_str("10").is_err());
        assert!(calculate_blake2b_length_str("520").is_err());
        assert_eq!(calculate_blake2b_length_str("512").unwrap(), None);
        assert_eq!(calculate_blake2b_length_str("256").unwrap(), Some(32));
    }
}
