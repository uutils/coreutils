// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore bitlen

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use crate::checksum::{ChecksumError, SizedAlgoKind, digest_reader, escape_filename};
use crate::error::{FromIo, UResult, USimpleError};
use crate::line_ending::LineEnding;
use crate::sum::DigestOutput;
use crate::{show, translate};

/// Use the same buffer size as GNU when reading a file to create a checksum
/// from it: 32 KiB.
const READ_BUFFER_SIZE: usize = 32 * 1024;

/// Necessary options when computing a checksum. Historically, these options
/// included a `binary` field to differentiate `--binary` and `--text` modes on
/// windows. Since the support for this feature is approximate in GNU, and it's
/// deprecated anyway, it was decided in #9168 to ignore the difference when
/// computing the checksum.
pub struct ChecksumComputeOptions {
    /// Which algorithm to use to compute the digest.
    pub algo_kind: SizedAlgoKind,

    /// Printing format to use for each checksum.
    pub output_format: OutputFormat,

    /// Whether to finish lines with '\n' or '\0'.
    pub line_ending: LineEnding,
}

/// Reading mode used to compute digest.
///
/// On most linux systems, this is irrelevant, as there is no distinction
/// between text and binary files. Refer to GNU's cksum documentation for more
/// information.
///
/// As discussed in #9168, we decide to ignore the reading mode to compute the
/// digest, both on Windows and UNIX. The reason for that is that this is a
/// legacy feature that is poorly documented and used. This enum is kept
/// nonetheless to still take into account the flags passed to cksum when
/// generating untagged lines.
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

/// Use already-processed arguments to decide the output format.
pub fn figure_out_output_format(
    algo: SizedAlgoKind,
    tag: bool,
    binary: bool,
    raw: bool,
    base64: bool,
) -> OutputFormat {
    // Raw output format takes precedence over anything else.
    if raw {
        return OutputFormat::Raw;
    }

    // Then, if the algo is legacy, takes precedence over the rest
    if algo.is_legacy() {
        return OutputFormat::Legacy;
    }

    let digest_format = if base64 {
        DigestFormat::Base64
    } else {
        DigestFormat::Hexadecimal
    };

    // After that, decide between tagged and untagged output
    if tag {
        OutputFormat::Tagged(digest_format)
    } else {
        let reading_mode = if binary {
            ReadingMode::Binary
        } else {
            ReadingMode::Text
        };
        OutputFormat::Untagged(digest_format, reading_mode)
    }
}

fn print_legacy_checksum(
    options: &ChecksumComputeOptions,
    filename: &OsStr,
    sum: &DigestOutput,
    size: usize,
) -> UResult<()> {
    debug_assert!(options.algo_kind.is_legacy());
    debug_assert!(matches!(sum, DigestOutput::U16(_) | DigestOutput::Crc(_)));

    let (escaped_filename, prefix) = if options.line_ending == LineEnding::Nul {
        (filename.to_string_lossy().to_string(), "")
    } else {
        escape_filename(filename)
    };

    // Print the sum
    match (options.algo_kind, sum) {
        (SizedAlgoKind::Sysv, DigestOutput::U16(sum)) => print!(
            "{prefix}{sum} {}",
            size.div_ceil(options.algo_kind.bitlen()),
        ),
        (SizedAlgoKind::Bsd, DigestOutput::U16(sum)) => {
            // The BSD checksum output is 5 digit integer
            let bsd_width = 5;
            print!(
                "{prefix}{sum:0bsd_width$} {:bsd_width$}",
                size.div_ceil(options.algo_kind.bitlen()),
            );
        }
        (SizedAlgoKind::Crc | SizedAlgoKind::Crc32b, DigestOutput::Crc(sum)) => {
            print!("{prefix}{sum} {size}");
        }
        (algo, output) => unreachable!("Bug: Invalid legacy checksum ({algo:?}, {output:?})"),
    }

    // Print the filename after a space if not stdin
    if escaped_filename != "-" {
        print!(" ");
        let _dropped_result = io::stdout().write_all(escaped_filename.as_bytes());
    }

    Ok(())
}

fn print_tagged_checksum(
    options: &ChecksumComputeOptions,
    filename: &OsStr,
    sum: &String,
) -> UResult<()> {
    let (escaped_filename, prefix) = if options.line_ending == LineEnding::Nul {
        (filename.to_string_lossy().to_string(), "")
    } else {
        escape_filename(filename)
    };

    // Print algo name and opening parenthesis.
    print!("{prefix}{} (", options.algo_kind.to_tag());

    // Print filename
    let _dropped_result = io::stdout().write_all(escaped_filename.as_bytes());

    // Print closing parenthesis and sum
    print!(") = {sum}");

    Ok(())
}

fn print_untagged_checksum(
    options: &ChecksumComputeOptions,
    filename: &OsStr,
    sum: &String,
    reading_mode: ReadingMode,
) -> UResult<()> {
    let (escaped_filename, prefix) = if options.line_ending == LineEnding::Nul {
        (filename.to_string_lossy().to_string(), "")
    } else {
        escape_filename(filename)
    };

    // Print checksum and reading mode flag
    print!("{prefix}{sum} {}", reading_mode.as_char());

    // Print filename
    let _dropped_result = io::stdout().write_all(escaped_filename.as_bytes());

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
                translate!("error-is-a-directory", "file" => filepath.display())
            ));
            continue;
        }

        // Handle the file input
        let mut file = BufReader::with_capacity(
            READ_BUFFER_SIZE,
            if filename == "-" {
                stdin_buf = io::stdin();
                Box::new(stdin_buf) as Box<dyn Read>
            } else {
                file_buf = match File::open(filepath) {
                    Ok(file) => file,
                    Err(err) => {
                        show!(err.map_err_context(|| filepath.to_string_lossy().into()));
                        continue;
                    }
                };
                Box::new(file_buf) as Box<dyn Read>
            },
        );

        let mut digest = options.algo_kind.create_digest();

        // Always compute the "binary" version of the digest, i.e. on Windows,
        // never handle CRLFs specifically.
        let (digest_output, sz) = digest_reader(&mut digest, &mut file, /* binary: */ true)
            .map_err_context(|| translate!("checksum-error-failed-to-read-input"))?;

        // Encodes the sum if df is Base64, leaves as-is otherwise.
        let encode_sum = |sum: DigestOutput, df: DigestFormat| {
            if df.is_base64() {
                sum.to_base64()
            } else {
                sum.to_hex()
            }
        };

        match options.output_format {
            OutputFormat::Raw => {
                // Cannot handle multiple files anyway, output immediately.
                digest_output.write_raw(io::stdout())?;
                return Ok(());
            }
            OutputFormat::Legacy => {
                print_legacy_checksum(&options, filename, &digest_output, sz)?;
            }
            OutputFormat::Tagged(digest_format) => {
                print_tagged_checksum(
                    &options,
                    filename,
                    &encode_sum(digest_output, digest_format)?,
                )?;
            }
            OutputFormat::Untagged(digest_format, reading_mode) => {
                print_untagged_checksum(
                    &options,
                    filename,
                    &encode_sum(digest_output, digest_format)?,
                    reading_mode,
                )?;
            }
        }

        print!("{}", options.line_ending);
    }
    Ok(())
}
