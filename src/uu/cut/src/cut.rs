// This file is part of the uutils coreutils package.
//
// (c) Rolf Morel <rolfmorel@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim sourcefiles

/* TODO
 *
 * - Implement actual cutting: At files, bytes and chars
 * - Implement file handling
 * - FIX: In cut_bytes iterator also joins empty matches --> wrong!
 */

#[macro_use]
extern crate uucore;

// use bstr::io::BufReadExt;
use clap::{crate_version, App, Arg, ArgMatches};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
// use uucore::display::Quotable;

use uucore::error::{UError, UIoError, UResult};
use uucore::ranges::Range;
use uucore::InvalidEncodingHandling;

/* ****************************************************************************
 * Help text and option definitions
 * ****************************************************************************/

static NAME: &str = "cut";
static ABOUT: &str = "Print selected parts of lines from each FILE to standard output.

With no FILE, or when FILE is -, read standard input.

Mandatory arguments to long options are mandatory for short options too.";
static AFTER_HELP: &str = "Use one, and only one of -b, -c or -f.  Each LIST is made up of one
range, or many ranges separated by commas.  Selected input is written
in the same order that it is read, and is written exactly once.
Each range is one of:

  N     N'th byte, character or field, counted from 1
  N-    from N'th byte, character or field, to end of line
  N-M   from N'th to M'th (included) byte, character or field
  -M    from first to M'th (included) byte, character or field

";

mod options {
    // Flags
    pub const COMPLEMENT: &str = "complement";
    pub const DONT_SPLIT_MULTIBYTES: &str = "n";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    // Options
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    // File input
    pub const FILE: &str = "FILE";
}

/* ****************************************************************************
 * Error handling and custom error
 * ****************************************************************************/

#[derive(Debug)]
enum CutError {
    OnlyOneListAllowed(),
    NeedOneList(),
    InvalidFieldList(String),
    InvalidByteCharList(String),
    InputDelimOnlyOnFields(),
    IsDirectory(PathBuf),
    SuppressingOnlyOnFields(),
    DelimSingleChar(),
    NotImplemented(String),
}

impl UError for CutError {
    fn code(&self) -> i32 {
        1
    }

    fn usage(&self) -> bool {
        true
    }
}

impl Error for CutError {}

impl Display for CutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CutError as CE;
        match self {
            CE::OnlyOneListAllowed() => write!(f, "only one type of list may be specified"),
            CE::NeedOneList() => {
                write!(f, "you must specify a list of bytes, characters, or fields")
            }
            CE::InvalidFieldList(e) => {
                write!(f, "invalid field value '{}'", e)
            }
            CE::InvalidByteCharList(e) => {
                write!(f, "invalid byte/character position '{}'", e)
            }
            CE::InputDelimOnlyOnFields() => write!(
                f,
                "an input delimiter may be specified only when operating on fields"
            ),
            CE::IsDirectory(dir) => {
                write!(f, "{}: Is a directory", dir.display())
            },
            CE::SuppressingOnlyOnFields() => write!(
                f,
                "suppressing non-delimited lines makes sense\n        only when operating on fields"
            ),
            CE::DelimSingleChar() => write!(f, "the delimiter must be a single character"),
            CE::NotImplemented(thing) => write!(
                f, "'{}' isn't implemented yet.", thing
            )
        }
    }
}

/* ****************************************************************************
 * Custom data types and structures
 * ****************************************************************************/

enum Mode {
    Bytes(Vec<Range>),
    Characters(Vec<Range>),
    Fields(Vec<Range>, String),
}

struct Behavior {
    // Flags
    complement: bool,
    dont_split_multibytes: bool,
    only_delimited: bool,
    zero_terminated: bool,
    // Options
    mode: Mode,
    output_delimiter: Option<String>,
    // Files
    files: Vec<String>,
}

/* ****************************************************************************
 * Helper functions
 * ****************************************************************************/

fn stdout_writer() -> Box<dyn Write> {
    if atty::is(atty::Stream::Stdout) {
        Box::new(stdout())
    } else {
        Box::new(BufWriter::new(stdout())) as Box<dyn Write>
    }
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    if complement {
        Range::from_list(list).map(|r| uucore::ranges::complement(&r))
    } else {
        Range::from_list(list)
    }
}

fn cut_bytes(reader: impl Read, ranges: &Vec<Range>, opts: &Behavior) -> UResult<()> {
    let newline_char = if opts.zero_terminated { b'\0' } else { b'\n' };
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let delim = opts
        .output_delimiter
        .as_ref()
        .map_or("", String::as_str)
        .as_bytes();

    let mut buf = vec![];
    let mut out_buf: Vec<&[u8]> = vec![];

    loop {
        buf.clear();
        out_buf.clear();
        if buf_in.read_until(newline_char, &mut buf).unwrap() == 0 {
            // File read completely
            return Ok(());
        }

        if buf.is_empty() { continue };

        let out_bytes = ranges.iter()
            // Act only on ranges that make sense with this buffers length
            .filter(|range| { range.low < buf.len() })
            .map(|&Range { low, high }| {
                // Make sure we don't act on ranges that are out of bounds
                let l = (low - 1).min(buf.len());
                let h = high.min(buf.len());
                &buf[l..h]
            })
            .collect::<Vec<&[u8]>>()
            // Add the output delimiters between the slices
            .join(delim);
        if out_bytes.is_empty() { continue };

        out.write_all(&out_bytes[..]).unwrap();
        out.write_all(&[newline_char]).unwrap();
    }

    Ok(())
}

// #[allow(clippy::cognitive_complexity)]
// fn cut_fields_delimiter<R: Read>(
//     reader: R,
//     ranges: &[Range],
//     delim: &str,
//     only_delimited: bool,
//     newline_char: u8,
//     out_delim: &str,
// ) -> i32 {
//     let buf_in = BufReader::new(reader);
//     let mut out = stdout_writer();
//     let input_delim_len = delim.len();

//     let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
//         let mut fields_pos = 1;
//         let mut low_idx = 0;
//         let mut delim_search = Searcher::new(line, delim.as_bytes()).peekable();
//         let mut print_delim = false;

//         if delim_search.peek().is_none() {
//             if !only_delimited {
//                 out.write_all(line)?;
//                 if line[line.len() - 1] != newline_char {
//                     out.write_all(&[newline_char])?;
//                 }
//             }

//             return Ok(true);
//         }

//         for &Range { low, high } in ranges {
//             if low - fields_pos > 0 {
//                 low_idx = match delim_search.nth(low - fields_pos - 1) {
//                     Some(index) => index + input_delim_len,
//                     None => break,
//                 };
//             }

//             for _ in 0..=high - low {
//                 if print_delim {
//                     out.write_all(out_delim.as_bytes())?;
//                 } else {
//                     print_delim = true;
//                 }

//                 match delim_search.next() {
//                     Some(high_idx) => {
//                         let segment = &line[low_idx..high_idx];

//                         out.write_all(segment)?;

//                         low_idx = high_idx + input_delim_len;
//                         fields_pos = high + 1;
//                     }
//                     None => {
//                         let segment = &line[low_idx..];

//                         out.write_all(segment)?;

//                         if line[line.len() - 1] == newline_char {
//                             return Ok(true);
//                         }
//                         break;
//                     }
//                 }
//             }
//         }

//         out.write_all(&[newline_char])?;
//         Ok(true)
//     });
//     crash_if_err!(1, result);
//     0
// }

// #[allow(clippy::cognitive_complexity)]
// fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> i32 {
//     let newline_char = if opts.zero_terminated { b'\0' } else { b'\n' };
//     if let Some(ref o_delim) = opts.out_delimiter {
//         return cut_fields_delimiter(
//             reader,
//             ranges,
//             &opts.delimiter,
//             opts.only_delimited,
//             newline_char,
//             o_delim,
//         );
//     }

//     let buf_in = BufReader::new(reader);
//     let mut out = stdout_writer();
//     let delim_len = opts.delimiter.len();

//     let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
//         let mut fields_pos = 1;
//         let mut low_idx = 0;
//         let mut delim_search = Searcher::new(line, opts.delimiter.as_bytes()).peekable();
//         let mut print_delim = false;

//         if delim_search.peek().is_none() {
//             if !opts.only_delimited {
//                 out.write_all(line)?;
//                 if line[line.len() - 1] != newline_char {
//                     out.write_all(&[newline_char])?;
//                 }
//             }

//             return Ok(true);
//         }

//         for &Range { low, high } in ranges {
//             if low - fields_pos > 0 {
//                 if let Some(delim_pos) = delim_search.nth(low - fields_pos - 1) {
//                     low_idx = if print_delim {
//                         delim_pos
//                     } else {
//                         delim_pos + delim_len
//                     }
//                 } else {
//                     break;
//                 }
//             }

//             match delim_search.nth(high - low) {
//                 Some(high_idx) => {
//                     let segment = &line[low_idx..high_idx];

//                     out.write_all(segment)?;

//                     print_delim = true;
//                     low_idx = high_idx;
//                     fields_pos = high + 1;
//                 }
//                 None => {
//                     let segment = &line[low_idx..line.len()];

//                     out.write_all(segment)?;

//                     if line[line.len() - 1] == newline_char {
//                         return Ok(true);
//                     }
//                     break;
//                 }
//             }
//         }
//         out.write_all(&[newline_char])?;
//         Ok(true)
//     });
//     crash_if_err!(1, result);
//     0
// }

fn cut_files(opts: &Behavior) -> UResult<()> {
    for filename in &opts.files {
        if filename == "-" {
            // exit_code |= match mode {
            //     Mode::Bytes(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
            //     Mode::Characters(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
            //     Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            // };
            show_if_err!(match &opts.mode {
                Mode::Bytes(range) => cut_bytes(stdin(), range, opts),
                _ => return Err(CutError::NotImplemented(
                    String::from("Modes except Bytes")).into()),
            });

        } else {
            let path = Path::new(&filename[..]);

            if path.is_dir() {
                show!(CutError::IsDirectory(path.to_path_buf()));
                continue;
            }

            if path.metadata().is_err() {
                show!(UIoError::new(
                    std::io::ErrorKind::NotFound,
                    format!("{}", path.display())
                ));
                continue;
            }

            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    show!(uio_error!(e, "opening {}", path.display()));
                    continue;
                }
            };

            show_if_err!(match &opts.mode {
                Mode::Bytes(range) => cut_bytes(file, range, opts),
                _ => return Err(CutError::NotImplemented(
                    String::from("Modes except Bytes")).into()),
            });
        }
    }

    Ok(())
}

fn get_behavior(matches: &ArgMatches) -> UResult<Behavior> {
    let complement = matches.is_present(options::COMPLEMENT);

    // Option sanity checks: Check for mutually exclusive options before
    // processing any further
    if matches.is_present(options::BYTES) | matches.is_present(options::CHARACTERS) {
        if matches.is_present(options::DELIMITER) {
            return Err(CutError::InputDelimOnlyOnFields().into());
        }
        if matches.is_present(options::ONLY_DELIMITED) {
            return Err(CutError::SuppressingOnlyOnFields().into());
        }
    }
    // Presence of '-n' is currently completely ignored it seems.

    let mode = match (
        matches.value_of(options::BYTES),
        matches.value_of(options::CHARACTERS),
        matches.value_of(options::FIELDS),
    ) {
        (Some(byte_ranges), None, None) => {
            // TODO: Option "-n"
            let ranges = list_to_ranges(byte_ranges, complement)
                .map_err(|_| CutError::InvalidByteCharList(byte_ranges.to_string()))?;
            Mode::Bytes(ranges)
        }
        (None, Some(char_ranges), None) => {
            let ranges = list_to_ranges(char_ranges, complement)
                .map_err(|_| CutError::InvalidByteCharList(char_ranges.to_string()))?;
            Mode::Characters(ranges)
        }
        (None, None, Some(field_ranges)) => {
            let ranges = list_to_ranges(field_ranges, complement)
                .map_err(|_| CutError::InvalidFieldList(field_ranges.to_string()))?;
            let field_delim = String::from(matches.value_of(options::DELIMITER).unwrap_or("\t"));
            Mode::Fields(ranges, field_delim)
        }
        (None, None, None) => return Err(CutError::NeedOneList().into()),
        _ => return Err(CutError::OnlyOneListAllowed().into()),
    };

    let output_delimiter = Some(" ".to_owned());

    let files: Vec<String> = matches
        .values_of(options::FILE)
        .unwrap_or_default()
        .map(str::to_owned)
        .collect();

    Ok(Behavior {
        // Flags
        complement: matches.is_present(options::COMPLEMENT),
        dont_split_multibytes: matches.is_present(options::DONT_SPLIT_MULTIBYTES),
        only_delimited: matches.is_present(options::ONLY_DELIMITED),
        zero_terminated: matches.is_present(options::ZERO_TERMINATED),
        // Options
        mode,
        output_delimiter,
        // Files
        files,
    })
}

/* ****************************************************************************
 * Main routine
 * ****************************************************************************/

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);
    let behavior = get_behavior(&matches)?;

    // match (behavior.mode) {
    //     Mode::Bytes(_) => cut_bytes(&behavior),
    //     Mode::Characters(_) => cut_characters(&behavior),
    //     Mode::Fields(_) => cut_fields(&behavior),
    // }
    return Ok(());
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
            Arg::with_name(options::BYTES)
                .short("b")
                .long(options::BYTES)
                .takes_value(true)
                .help("select only these bytes")
                .allow_hyphen_values(true)
                .value_name("LIST"),
        )
        .arg(
            Arg::with_name(options::CHARACTERS)
                .short("c")
                .long(options::CHARACTERS)
                .help("select only these characters")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST"),
        )
        .arg(
            Arg::with_name(options::DELIMITER)
                .short("d")
                .long(options::DELIMITER)
                .help("use DELIM instead of TAB for field delimiter")
                .takes_value(true)
                .value_name("DELIM"),
        )
        .arg(
            Arg::with_name(options::FIELDS)
                .short("f")
                .long(options::FIELDS)
                .help(
                    "select only these fields;  also print any line
  that contains no delimiter character, unless
  the -s option is specified",
                )
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("LIST"),
        )
        .arg(
            Arg::with_name(options::DONT_SPLIT_MULTIBYTES)
                .short(options::DONT_SPLIT_MULTIBYTES)
                .help("with -b: don't split multibyte characters")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help(
                    "complement the set of selected bytes, characters
  or fields",
                )
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::ONLY_DELIMITED)
                .short("s")
                .long(options::ONLY_DELIMITED)
                .help("do not print lines not containing delimiters")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::OUTPUT_DELIMITER)
                .long(options::OUTPUT_DELIMITER)
                .help(
                    "use STRING as the output delimiter
  the default is to use the input delimiter",
                )
                .takes_value(true)
                .value_name("STRING")
                .default_value("")
        )
        .arg(
            Arg::with_name(options::ZERO_TERMINATED)
                .short("z")
                .long(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::FILE)
                .hidden(false)
                .multiple(true)
                .default_value("-")
        )
}
