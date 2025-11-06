use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use crate::checksum::{ChecksumError, SizedAlgoKind, digest_reader};
use crate::error::{FromIo, UResult, USimpleError};
use crate::line_ending::LineEnding;
use crate::{encoding, os_str_as_bytes, show, translate};

pub struct ChecksumComputeOptions {
    pub algo_kind: SizedAlgoKind,
    pub output_format: OutputFormat,
    pub line_ending: LineEnding,
}

/// Reading mode used to compute digest.
///
/// On most linux systems, this is irrelevant, as there is no distinction
/// between text and binary files. Refer to GNU's cksum documentation for more
/// information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingMode {
    Binary,
    Text,
}

impl ReadingMode {
    #[inline]
    fn as_char(&self) -> char {
        match self {
            Self::Binary => '*',
            Self::Text => ' ',
        }
    }
}

/// Whether to write the digest as hexadecimal or encoded in base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestFormat {
    Hexadecimal,
    Base64,
}

impl DigestFormat {
    #[inline]
    fn is_base64(&self) -> bool {
        *self == Self::Base64
    }
}

/// Holds the representation that shall be used for printing a checksum line
#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Raw digest
    Raw,

    /// Selected for older algorithms which had their custom formatting
    ///
    /// Default for crc, sysv, bsd
    Legacy,

    /// `$ALGO_NAME ($FILENAME) = $DIGEST`
    Tagged(DigestFormat),

    /// '$DIGEST $FLAG$FILENAME'
    /// where 'flag' depends on the reading mode
    ///
    /// Default for standalone checksum utilities
    Untagged(DigestFormat, ReadingMode),
}

impl OutputFormat {
    #[inline]
    fn is_raw(&self) -> bool {
        *self == Self::Raw
    }
}

fn print_legacy_checksum(
    options: &ChecksumComputeOptions,
    filename: &OsStr,
    sum: &str,
    size: usize,
) -> UResult<()> {
    debug_assert!(options.algo_kind.is_legacy());

    // Print the sum
    match options.algo_kind {
        SizedAlgoKind::Sysv => print!(
            "{} {}",
            sum.parse::<u16>().unwrap(),
            size.div_ceil(options.algo_kind.bitlen()),
        ),
        SizedAlgoKind::Bsd => {
            // The BSD checksum output is 5 digit integer
            let bsd_width = 5;
            print!(
                "{:0bsd_width$} {:bsd_width$}",
                sum.parse::<u16>().unwrap(),
                size.div_ceil(options.algo_kind.bitlen()),
            );
        }
        SizedAlgoKind::Crc | SizedAlgoKind::Crc32b => {
            print!("{sum} {size}");
        }
        _ => unreachable!("Not a legacy algorithm"),
    }

    // Print the filename after a space if not stdin
    if filename != "-" {
        print!(" ");
        let _dropped_result = io::stdout().write_all(os_str_as_bytes(filename)?);
    }

    Ok(())
}

fn print_tagged_checksum(
    options: &ChecksumComputeOptions,
    filename: &OsStr,
    sum: &String,
) -> UResult<()> {
    // Print algo name and opening parenthesis.
    print!("{} (", options.algo_kind.to_tag());

    // Print filename
    let _dropped_result = io::stdout().write_all(os_str_as_bytes(filename)?);

    // Print closing parenthesis and sum
    print!(") = {sum}");

    Ok(())
}

fn print_untagged_checksum(
    filename: &OsStr,
    sum: &String,
    reading_mode: ReadingMode,
) -> UResult<()> {
    // Print checksum and reading mode flag
    print!("{sum} {}", reading_mode.as_char());

    // Print filename
    let _dropped_result = io::stdout().write_all(os_str_as_bytes(filename)?);

    Ok(())
}

/// Calculate checksum
///
/// # Arguments
///
/// * `options` - CLI options for the assigning checksum algorithm
/// * `files` - A iterator of [`OsStr`] which is a bunch of files that are using for calculating checksum
pub fn perform_checksum_computation<'a, I>(options: ChecksumComputeOptions, files: I) -> UResult<()>
where
    I: Iterator<Item = &'a OsStr>,
{
    let mut files = files.peekable();

    while let Some(filename) = files.next() {
        // Check that in raw mode, we are not provided with several files.
        if options.output_format.is_raw() && files.peek().is_some() {
            return Err(Box::new(ChecksumError::RawMultipleFiles));
        }

        let filepath = Path::new(filename);
        let stdin_buf;
        let file_buf;
        if filepath.is_dir() {
            show!(USimpleError::new(
                1,
                translate!("cksum-error-is-directory", "file" => filepath.display())
            ));
            continue;
        }

        // Handle the file input
        let mut file = BufReader::new(if filename == "-" {
            stdin_buf = io::stdin();
            Box::new(stdin_buf) as Box<dyn Read>
        } else {
            file_buf = match File::open(filepath) {
                Ok(file) => file,
                Err(err) => {
                    show!(err.map_err_context(|| filepath.to_string_lossy().to_string()));
                    continue;
                }
            };
            Box::new(file_buf) as Box<dyn Read>
        });

        let mut digest = options.algo_kind.create_digest();

        let (sum_hex, sz) = digest_reader(
            &mut digest,
            &mut file,
            false,
            options.algo_kind.bitlen(),
        )
        .map_err_context(|| translate!("cksum-error-failed-to-read-input"))?;

        // Encodes the sum if df is Base64, leaves as-is otherwise.
        let encode_sum = |sum: String, df: DigestFormat| {
            if df.is_base64() {
                encoding::for_cksum::BASE64.encode(&hex::decode(sum).unwrap())
            } else {
                sum
            }
        };

        match options.output_format {
            OutputFormat::Raw => {
                let bytes = match options.algo_kind {
                    SizedAlgoKind::Crc | SizedAlgoKind::Crc32b => {
                        sum_hex.parse::<u32>().unwrap().to_be_bytes().to_vec()
                    }
                    SizedAlgoKind::Sysv | SizedAlgoKind::Bsd => {
                        sum_hex.parse::<u16>().unwrap().to_be_bytes().to_vec()
                    }
                    _ => hex::decode(sum_hex).unwrap(),
                };
                // Cannot handle multiple files anyway, output immediately.
                io::stdout().write_all(&bytes)?;
                return Ok(());
            }
            OutputFormat::Legacy => {
                print_legacy_checksum(&options, filename, &sum_hex, sz)?;
            }
            OutputFormat::Tagged(digest_format) => {
                print_tagged_checksum(&options, filename, &encode_sum(sum_hex, digest_format))?;
            }
            OutputFormat::Untagged(digest_format, reading_mode) => {
                print_untagged_checksum(
                    filename,
                    &encode_sum(sum_hex, digest_format),
                    reading_mode,
                )?;
            }
        }

        print!("{}", options.line_ending);
    }
    Ok(())
}
