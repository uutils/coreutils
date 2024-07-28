// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore anotherfile invalidchecksum regexes JWZG

use data_encoding::BASE64;
use os_display::Quotable;
use regex::Regex;
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use crate::{
    error::{set_exit_code, FromIo, UError, UResult, USimpleError},
    show, show_error, show_warning_caps,
    sum::{
        Blake2b, Blake3, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha3_224,
        Sha3_256, Sha3_384, Sha3_512, Sha512, Shake128, Shake256, Sm3, BSD, CRC, SYSV,
    },
    util_name,
};
use std::io::stdin;
use std::io::BufRead;
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

#[derive(Default)]
struct ChecksumResult {
    pub bad_format: i32,
    pub failed_cksum: i32,
    pub failed_open_file: i32,
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
fn cksum_output(res: &ChecksumResult, ignore_missing: bool, status: bool) {
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
    if !ignore_missing {
        if res.failed_open_file == 1 {
            show_warning_caps!("{} listed file could not be read", res.failed_open_file);
        } else if res.failed_open_file > 1 {
            show_warning_caps!("{} listed files could not be read", res.failed_open_file);
        }
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
const ALGO_BASED_REGEX: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>.*)\)\s*=\s*(?P<checksum>[a-fA-F0-9]+)$";
const ALGO_BASED_REGEX_BASE64: &str = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename>.*)\)\s*=\s*(?P<checksum>[A-Za-z0-9+/]+={0,2})$";

const DOUBLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s{2}(?P<filename>.*)$";

// In this case, we ignore the *
const SINGLE_SPACE_REGEX: &str = r"^(?P<checksum>[a-fA-F0-9]+)\s(?P<filename>\*?.*)$";

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
fn determine_regex(lines: &[String]) -> Option<(Regex, bool)> {
    let regexes = [
        (Regex::new(ALGO_BASED_REGEX).unwrap(), true),
        (Regex::new(DOUBLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(SINGLE_SPACE_REGEX).unwrap(), false),
        (Regex::new(ALGO_BASED_REGEX_BASE64).unwrap(), true),
    ];

    for line in lines {
        let line_trim = line.trim();
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
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}

fn get_expected_checksum(
    filename: &str,
    caps: &regex::Captures,
    chosen_regex: &Regex,
) -> UResult<String> {
    if chosen_regex.as_str() == ALGO_BASED_REGEX_BASE64 {
        let ck = caps.name("checksum").unwrap().as_str();
        match BASE64.decode(ck.as_bytes()) {
            Ok(decoded_bytes) => {
                match std::str::from_utf8(&decoded_bytes) {
                    Ok(decoded_str) => Ok(decoded_str.to_string()),
                    Err(_) => Ok(bytes_to_hex(&decoded_bytes)), // Handle as raw bytes if not valid UTF-8
                }
            }
            Err(_) => Err(Box::new(
                ChecksumError::NoProperlyFormattedChecksumLinesFound {
                    filename: (&filename).to_string(),
                },
            )),
        }
    } else {
        Ok(caps.name("checksum").unwrap().as_str().to_string())
    }
}

/// Returns a reader that reads from the specified file, or from stdin if `filename_to_check` is "-".
fn get_file_to_check(
    filename: &str,
    ignore_missing: bool,
    res: &mut ChecksumResult,
) -> Option<Box<dyn Read>> {
    if filename == "-" {
        Some(Box::new(stdin())) // Use stdin if "-" is specified in the checksum file
    } else {
        match File::open(filename) {
            Ok(f) => {
                if f.metadata().ok()?.is_dir() {
                    show!(USimpleError::new(
                        1,
                        format!("{}: Is a directory", filename)
                    ));
                    None
                } else {
                    Some(Box::new(f))
                }
            }
            Err(err) => {
                if !ignore_missing {
                    // yes, we have both stderr and stdout here
                    show!(err.map_err_context(|| filename.to_string()));
                    println!("{}: FAILED open or read", filename);
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
    caps: &regex::Captures,
    algo_name_input: Option<&str>,
    res: &mut ChecksumResult,
    properly_formatted: &mut bool,
) -> Option<(String, Option<usize>)> {
    // When the algo-based format is matched, extract details from regex captures
    let algorithm = caps.name("algo").map_or("", |m| m.as_str()).to_lowercase();

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
        let bits_value = m.as_str().parse::<usize>().unwrap();
        if bits_value % 8 == 0 {
            Some(Some(bits_value / 8))
        } else {
            *properly_formatted = false;
            None // Return None to signal a divisibility issue
        }
    })?;

    Some((algorithm, bits))
}

/***
 * Do the checksum validation (can be strict or not)
*/
#[allow(clippy::too_many_arguments)]
pub fn perform_checksum_validation<'a, I>(
    files: I,
    strict: bool,
    status: bool,
    warn: bool,
    binary: bool,
    ignore_missing: bool,
    quiet: bool,
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
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
                    continue;
                }
            }
        };

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        let Some((chosen_regex, is_algo_based_format)) = determine_regex(&lines) else {
            let e = ChecksumError::NoProperlyFormattedChecksumLinesFound {
                filename: get_filename_for_output(filename_input, input_is_stdin),
            };
            show_error!("{e}");
            set_exit_code(1);
            continue;
        };

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = chosen_regex.captures(line) {
                properly_formatted = true;

                let mut filename_to_check = caps.name("filename").unwrap().as_str();
                if filename_to_check.starts_with('*')
                    && i == 0
                    && chosen_regex.as_str() == SINGLE_SPACE_REGEX
                {
                    // Remove the leading asterisk if present - only for the first line
                    filename_to_check = &filename_to_check[1..];
                }

                let expected_checksum =
                    get_expected_checksum(filename_to_check, &caps, &chosen_regex)?;

                // If the algo_name is provided, we use it, otherwise we try to detect it
                let (algo_name, length) = if is_algo_based_format {
                    identify_algo_name_and_length(
                        &caps,
                        algo_name_input,
                        &mut res,
                        &mut properly_formatted,
                    )
                    .unwrap_or((String::new(), None))
                } else if let Some(a) = algo_name_input {
                    // When a specific algorithm name is input, use it and use the provided bits
                    // except when dealing with blake2b, where we will detect the length
                    if algo_name_input == Some(ALGORITHM_OPTIONS_BLAKE2B) {
                        // division by 2 converts the length of the Blake2b checksum from hexadecimal
                        // characters to bytes, as each byte is represented by two hexadecimal characters.
                        let length = Some(expected_checksum.len() / 2);
                        (ALGORITHM_OPTIONS_BLAKE2B.to_string(), length)
                    } else {
                        (a.to_lowercase(), length_input)
                    }
                } else {
                    // Default case if no algorithm is specified and non-algo based format is matched
                    (String::new(), None)
                };

                if algo_name.is_empty() {
                    // we haven't been able to detect the algo name. No point to continue
                    properly_formatted = false;
                    continue;
                }
                let mut algo = detect_algo(&algo_name, length)?;

                let (filename_to_check_unescaped, prefix) = unescape_filename(filename_to_check);

                // manage the input file
                let file_to_check =
                    match get_file_to_check(&filename_to_check_unescaped, ignore_missing, &mut res)
                    {
                        Some(file) => file,
                        None => continue,
                    };
                let mut file_reader = BufReader::new(file_to_check);
                // Read the file and calculate the checksum
                let create_fn = &mut algo.create_fn;
                let mut digest = create_fn();
                let (calculated_checksum, _) =
                    digest_reader(&mut digest, &mut file_reader, binary, algo.bits).unwrap();

                // Do the checksum validation
                if expected_checksum == calculated_checksum {
                    if !quiet && !status {
                        println!("{prefix}{filename_to_check}: OK");
                    }
                    correct_format += 1;
                } else {
                    if !status {
                        println!("{prefix}{filename_to_check}: FAILED");
                    }
                    res.failed_cksum += 1;
                }
            } else {
                if line.is_empty() {
                    // Don't show any warning for empty lines
                    continue;
                }
                if warn {
                    let algo = if let Some(algo_name_input) = algo_name_input {
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
        }

        // not a single line correctly formatted found
        // return an error
        if !properly_formatted {
            if !status {
                return Err(ChecksumError::NoProperlyFormattedChecksumLinesFound {
                    filename: get_filename_for_output(filename_input, input_is_stdin),
                }
                .into());
            }
            set_exit_code(1);

            return Ok(());
        }

        if ignore_missing && correct_format == 0 {
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
        if strict && res.bad_format > 0 {
            set_exit_code(1);
        }

        // if we have any failed checksum verification, we set an exit code
        // except if we have ignore_missing
        if (res.failed_cksum > 0 || res.failed_open_file > 0) && !ignore_missing {
            set_exit_code(1);
        }

        // if any incorrectly formatted line, show it
        cksum_output(&res, ignore_missing, status);
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

pub fn unescape_filename(filename: &str) -> (String, &'static str) {
    let unescaped = filename
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
        .replace("\\r", "\r");
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

    #[test]
    fn test_unescape_filename() {
        let (unescaped, prefix) = unescape_filename("test\\nfile.txt");
        assert_eq!(unescaped, "test\nfile.txt");
        assert_eq!(prefix, "\\");
        let (unescaped, prefix) = unescape_filename("test\\nfile.txt");
        assert_eq!(unescaped, "test\nfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename("test\\rfile.txt");
        assert_eq!(unescaped, "test\rfile.txt");
        assert_eq!(prefix, "\\");

        let (unescaped, prefix) = unescape_filename("test\\\\file.txt");
        assert_eq!(unescaped, "test\\file.txt");
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
        let test_cases = vec![
            ("SHA256 (example.txt) = d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2", Some(("SHA256", None, "example.txt", "d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2"))),
            // cspell:disable-next-line
            ("BLAKE2b-512 (file) = abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef", Some(("BLAKE2b", Some("512"), "file", "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef"))),
            (" MD5 (test) = 9e107d9d372bb6826bd81d3542a419d6", Some(("MD5", None, "test", "9e107d9d372bb6826bd81d3542a419d6"))),
            ("SHA-1 (anotherfile) = a9993e364706816aba3e25717850c26c9cd0d89d", Some(("SHA", Some("1"), "anotherfile", "a9993e364706816aba3e25717850c26c9cd0d89d"))),
        ];

        for (input, expected) in test_cases {
            let captures = algo_based_regex.captures(input);
            match expected {
                Some((algo, bits, filename, checksum)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(captures.name("algo").unwrap().as_str(), algo);
                    assert_eq!(captures.name("bits").map(|m| m.as_str()), bits);
                    assert_eq!(captures.name("filename").unwrap().as_str(), filename);
                    assert_eq!(captures.name("checksum").unwrap().as_str(), checksum);
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

        let test_cases = vec![
            (
                "60b725f10c9c85c70d97880dfe8191b3  a",
                Some(("60b725f10c9c85c70d97880dfe8191b3", "a")),
            ),
            (
                "bf35d7536c785cf06730d5a40301eba2   b",
                Some(("bf35d7536c785cf06730d5a40301eba2", " b")),
            ),
            (
                "f5b61709718c1ecf8db1aea8547d4698  *c",
                Some(("f5b61709718c1ecf8db1aea8547d4698", "*c")),
            ),
            (
                "b064a020db8018f18ff5ae367d01b212  dd",
                Some(("b064a020db8018f18ff5ae367d01b212", "dd")),
            ),
            (
                "b064a020db8018f18ff5ae367d01b212   ",
                Some(("b064a020db8018f18ff5ae367d01b212", " ")),
            ),
            ("invalidchecksum  test", None),
        ];

        for (input, expected) in test_cases {
            let captures = double_space_regex.captures(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(captures.name("checksum").unwrap().as_str(), checksum);
                    assert_eq!(captures.name("filename").unwrap().as_str(), filename);
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
        let test_cases = vec![
            (
                "60b725f10c9c85c70d97880dfe8191b3 a",
                Some(("60b725f10c9c85c70d97880dfe8191b3", "a")),
            ),
            (
                "bf35d7536c785cf06730d5a40301eba2 b",
                Some(("bf35d7536c785cf06730d5a40301eba2", "b")),
            ),
            (
                "f5b61709718c1ecf8db1aea8547d4698 *c",
                Some(("f5b61709718c1ecf8db1aea8547d4698", "*c")),
            ),
            (
                "b064a020db8018f18ff5ae367d01b212 dd",
                Some(("b064a020db8018f18ff5ae367d01b212", "dd")),
            ),
            ("invalidchecksum test", None),
        ];

        for (input, expected) in test_cases {
            let captures = single_space_regex.captures(input);
            match expected {
                Some((checksum, filename)) => {
                    assert!(captures.is_some());
                    let captures = captures.unwrap();
                    assert_eq!(captures.name("checksum").unwrap().as_str(), checksum);
                    assert_eq!(captures.name("filename").unwrap().as_str(), filename);
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
        let lines_algo_based =
            vec!["MD5 (example.txt) = d41d8cd98f00b204e9800998ecf8427e".to_string()];
        let (regex, algo_based) = determine_regex(&lines_algo_based).unwrap();
        assert!(algo_based);
        assert!(regex.is_match(&lines_algo_based[0]));

        // Test double-space regex
        let lines_double_space = vec!["d41d8cd98f00b204e9800998ecf8427e  example.txt".to_string()];
        let (regex, algo_based) = determine_regex(&lines_double_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(&lines_double_space[0]));

        // Test single-space regex
        let lines_single_space = vec!["d41d8cd98f00b204e9800998ecf8427e example.txt".to_string()];
        let (regex, algo_based) = determine_regex(&lines_single_space).unwrap();
        assert!(!algo_based);
        assert!(regex.is_match(&lines_single_space[0]));

        // Test double-space regex start with invalid
        let lines_double_space = vec![
            "ERR".to_string(),
            "d41d8cd98f00b204e9800998ecf8427e  example.txt".to_string(),
        ];
        let (regex, algo_based) = determine_regex(&lines_double_space).unwrap();
        assert!(!algo_based);
        assert!(!regex.is_match(&lines_double_space[0]));
        assert!(regex.is_match(&lines_double_space[1]));

        // Test invalid checksum line
        let lines_invalid = vec!["invalid checksum line".to_string()];
        assert!(determine_regex(&lines_invalid).is_none());
    }

    #[test]
    fn test_get_expected_checksum() {
        let re = Regex::new(ALGO_BASED_REGEX_BASE64).unwrap();
        let caps = re
            .captures("SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=")
            .unwrap();

        let result = get_expected_checksum("filename", &caps, &re);

        assert_eq!(
            result.unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_get_expected_checksum_invalid() {
        let re = Regex::new(ALGO_BASED_REGEX_BASE64).unwrap();
        let caps = re
            .captures("SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU")
            .unwrap();

        let result = get_expected_checksum("filename", &caps, &re);

        assert!(result.is_err());
    }
}
