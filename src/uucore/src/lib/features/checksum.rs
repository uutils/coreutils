// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use os_display::Quotable;
use regex::Regex;
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use crate::{
    error::{set_exit_code, FromIo, UResult, USimpleError},
    show, show_error, show_warning_caps,
    sum::{
        Blake2b, Blake3, Digest, DigestWriter, Md5, Sha1, Sha224, Sha256, Sha384, Sha3_224,
        Sha3_256, Sha3_384, Sha3_512, Sha512, Shake128, Shake256, Sm3, BSD, CRC, SYSV,
    },
    util_name,
};
use std::io::stdin;
use std::io::BufRead;

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

pub const SUPPORTED_ALGO: [&str; 15] = [
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

        Some(_) => Err(USimpleError::new(
            1,
            "Invalid output size for SHA3 (expected 224, 256, 384, or 512)",
        )),
        None => Err(USimpleError::new(1, "--bits required for SHA3")),
    }
}

#[allow(clippy::comparison_chain)]
pub fn cksum_output(
    bad_format: i32,
    failed_cksum: i32,
    failed_open_file: i32,
    ignore_missing: bool,
    status: bool,
) {
    if bad_format == 1 {
        show_warning_caps!("{} line is improperly formatted", bad_format);
    } else if bad_format > 1 {
        show_warning_caps!("{} lines are improperly formatted", bad_format);
    }

    if !status {
        if failed_cksum == 1 {
            show_warning_caps!("{} computed checksum did NOT match", failed_cksum);
        } else if failed_cksum > 1 {
            show_warning_caps!("{} computed checksums did NOT match", failed_cksum);
        }
    }
    if !ignore_missing {
        if failed_open_file == 1 {
            show_warning_caps!("{} listed file could not be read", failed_open_file);
        } else if failed_open_file > 1 {
            show_warning_caps!("{} listed files could not be read", failed_open_file);
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
        alg if alg.starts_with("sha3") => create_sha3(length),

        _ => Err(USimpleError::new(
            1,
            "unknown algorithm: clap should have prevented this case",
        )),
    }
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
    algo_name_input: Option<&str>,
    length_input: Option<usize>,
) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    // Regexp to handle the two input formats:
    // 1. <algo>[-<bits>] (<filename>) = <checksum>
    //    algo must be uppercase or b (for blake2b)
    // 2. <checksum> [* ]<filename>
    let regex_pattern = r"^\s*\\?(?P<algo>(?:[A-Z0-9]+|BLAKE2b))(?:-(?P<bits>\d+))?\s?\((?P<filename1>.*)\)\s*=\s*(?P<checksum1>[a-fA-F0-9]+)$|^(?P<checksum2>[a-fA-F0-9]+)\s[* ](?P<filename2>.*)";
    let re = Regex::new(regex_pattern).unwrap();

    // if cksum has several input files, it will print the result for each file
    for filename_input in files {
        let mut bad_format = 0;
        let mut failed_cksum = 0;
        let mut failed_open_file = 0;
        let mut correct_format = 0;
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
        for (i, line) in reader.lines().enumerate() {
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
                let (algo_name, length) = if algo_based_format {
                    // When the algo-based format is matched, extract details from regex captures
                    let algorithm = caps.name("algo").map_or("", |m| m.as_str()).to_lowercase();
                    if !SUPPORTED_ALGO.contains(&algorithm.as_str()) {
                        // Not supported algo, leave early
                        properly_formatted = false;
                        continue;
                    }

                    let bits = caps.name("bits").map_or(Some(None), |m| {
                        let bits_value = m.as_str().parse::<usize>().unwrap();
                        if bits_value % 8 == 0 {
                            Some(Some(bits_value / 8))
                        } else {
                            properly_formatted = false;
                            None // Return None to signal a parsing or divisibility issue
                        }
                    });

                    if bits.is_none() {
                        // If bits is None, we have a parsing or divisibility issue
                        // Exit the loop outside of the closure
                        continue;
                    }

                    (algorithm, bits.unwrap())
                } else if let Some(a) = algo_name_input {
                    // When a specific algorithm name is input, use it and default bits to None
                    (a.to_lowercase(), length_input)
                } else {
                    // Default case if no algorithm is specified and non-algo based format is matched
                    (String::new(), None)
                };

                if algo_based_format && algo_name_input.map_or(false, |input| algo_name != input) {
                    bad_format += 1;
                    continue;
                }

                if algo_name.is_empty() {
                    // we haven't been able to detect the algo name. No point to continue
                    properly_formatted = false;
                    continue;
                }
                let mut algo = detect_algo(&algo_name, length)?;

                let (filename_to_check_unescaped, prefix) = unescape_filename(filename_to_check);

                // manage the input file
                let file_to_check: Box<dyn Read> = if filename_to_check == "-" {
                    Box::new(stdin()) // Use stdin if "-" is specified in the checksum file
                } else {
                    match File::open(&filename_to_check_unescaped) {
                        Ok(f) => {
                            if f.metadata()?.is_dir() {
                                show!(USimpleError::new(
                                    1,
                                    format!("{}: Is a directory", filename_to_check_unescaped)
                                ));
                                continue;
                            }
                            Box::new(f)
                        }
                        Err(err) => {
                            if !ignore_missing {
                                // yes, we have both stderr and stdout here
                                show!(err.map_err_context(|| filename_to_check.to_string()));
                                println!("{}: FAILED open or read", filename_to_check);
                            }
                            failed_open_file += 1;
                            // we could not open the file but we want to continue

                            continue;
                        }
                    }
                };

                let mut file_reader = BufReader::new(file_to_check);
                // Read the file and calculate the checksum
                let create_fn = &mut algo.create_fn;
                let mut digest = create_fn();
                let (calculated_checksum, _) =
                    digest_reader(&mut digest, &mut file_reader, binary, algo.bits).unwrap();

                // Do the checksum validation
                if expected_checksum == calculated_checksum {
                    println!("{prefix}{filename_to_check}: OK");
                    correct_format += 1;
                } else {
                    if !status {
                        println!("{prefix}{filename_to_check}: FAILED");
                    }
                    failed_cksum += 1;
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

                bad_format += 1;
            }
        }

        // not a single line correctly formatted found
        // return an error
        if !properly_formatted {
            let filename = filename_input.to_string_lossy();
            show_error!(
                "{}: no properly formatted checksum lines found",
                if input_is_stdin {
                    "standard input"
                } else {
                    &filename
                }
                .maybe_quote()
            );
            set_exit_code(1);
        }

        if ignore_missing && correct_format == 0 {
            // we have only bad format
            // and we had ignore-missing
            eprintln!(
                "{}: {}: no file was verified",
                util_name(),
                filename_input.maybe_quote(),
            );
            //skip_summary = true;
            set_exit_code(1);
        }

        // strict means that we should have an exit code.
        if strict && bad_format > 0 {
            set_exit_code(1);
        }

        // if we have any failed checksum verification, we set an exit code
        // except if we have ignore_missing
        if (failed_cksum > 0 || failed_open_file > 0) && !ignore_missing {
            set_exit_code(1);
        }

        // if any incorrectly formatted line, show it
        cksum_output(
            bad_format,
            failed_cksum,
            failed_open_file,
            ignore_missing,
            status,
        );
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

/// Calculates the length of the digest for the given algorithm.
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
}
