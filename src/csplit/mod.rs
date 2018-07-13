#![crate_name = "uu_csplit"]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate uucore;
extern crate getopts;
extern crate regex;
use getopts::Matches;
use std::fs::File;
use std::io::{self, BufReader};

pub mod csplit_impl;
pub mod split_name;
pub mod patterns;

static SYNTAX: &'static str = "[OPTION]... FILE PATTERN...";
static SUMMARY: &'static str = "split a file into sections determined by context lines";
static LONG_HELP: &'static str = "Output pieces of FILE separated by PATTERN(s) to files 'xx00', 'xx01', ..., and output byte counts of each piece to standard output.";

static SUFFIX_FORMAT_OPT: &'static str = "suffix-format";
static SUPPRESS_MATCHED_OPT: &'static str = "suppress-matched";
static DIGITS_OPT: &'static str = "digits";
static PREFIX_OPT: &'static str = "prefix";
static KEEP_FILES_OPT: &'static str = "keep-files";
static QUIET_OPT: &'static str = "quiet";
static ELIDE_EMPTY_FILES_OPT: &'static str = "elide-empty-files";

/// Errors thrown by the csplit command
#[derive(Debug, Fail)]
pub enum CsplitError {
    #[fail(display = "IO error: {}", _0)]
    IoError(io::Error),
    #[fail(display = "'{}': line number out of range", _0)]
    LineOutOfRange(String),
    #[fail(display = "'{}': line number out of range on repetition {}", _0, _1)]
    LineOutOfRangeOnRepetition(String, usize),
    #[fail(display = "'{}': match not found", _0)]
    MatchNotFound(String),
    #[fail(display = "'{}': match not found on repetition {}", _0, _1)]
    MatchNotFoundOnRepetition(String, usize),
    #[fail(display = "line number must be greater than zero")]
    LineNumberIsZero,
    #[fail(display = "line number '{}' is smaller than preceding line number, {}", _0, _1)]
    LineNumberSmallerThanPrevious(usize, usize),
    #[fail(display = "invalid pattern: {}", _0)]
    InvalidPattern(String),
    #[fail(display = "invalid number: '{}'", _0)]
    InvalidNumber(String),
    #[fail(display = "incorrect conversion specification in suffix")]
    SuffixFormatIncorrect,
    #[fail(display = "too many % conversion specifications in suffix")]
    SuffixFormatTooManyPercents,
}

impl From<io::Error> for CsplitError {
    fn from(error: io::Error) -> Self {
        CsplitError::IoError(error)
    }
}

/// Command line options for csplit.
pub struct CsplitOptions {
    split_name: split_name::SplitName,
    keep_files: bool,
    quiet: bool,
    elide_empty_files: bool,
    suppress_matched: bool,
}

impl CsplitOptions {
    fn new(matches: &Matches) -> CsplitOptions {
        let keep_files = matches.opt_present(KEEP_FILES_OPT);
        let quiet = matches.opt_present(QUIET_OPT);
        let elide_empty_files = matches.opt_present(ELIDE_EMPTY_FILES_OPT);
        let suppress_matched = matches.opt_present(SUPPRESS_MATCHED_OPT);

        CsplitOptions {
            split_name: crash_if_err!(
                1,
                split_name::SplitName::new(
                    matches.opt_str(PREFIX_OPT),
                    matches.opt_str(SUFFIX_FORMAT_OPT),
                    matches.opt_str(DIGITS_OPT)
                )
            ),
            keep_files,
            quiet,
            elide_empty_files,
            suppress_matched,
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optopt(
            "b",
            SUFFIX_FORMAT_OPT,
            "use sprintf FORMAT instead of %02d",
            "FORMAT",
        )
        .optopt("f", PREFIX_OPT, "use PREFIX instead of 'xx'", "PREFIX")
        .optflag("k", KEEP_FILES_OPT, "do not remove output files on errors")
        .optflag(
            "",
            SUPPRESS_MATCHED_OPT,
            "suppress the lines matching PATTERN",
        )
        .optopt(
            "n",
            DIGITS_OPT,
            "use specified number of digits instead of 2",
            "DIGITS",
        )
        .optflag("s", QUIET_OPT, "do not print counts of output file sizes")
        .optflag("z", ELIDE_EMPTY_FILES_OPT, "remove empty output files")
        .parse(args);

    // check for mandatory arguments
    if matches.free.is_empty() {
        disp_err!("missing operand");
        exit!(1);
    }
    if matches.free.len() == 1 {
        disp_err!("missing operand after '{}'", matches.free[0]);
        exit!(1);
    }
    // get the patterns to split on
    let patterns = return_if_err!(1, patterns::get_patterns(&matches.free[1..]));
    // get the file to split
    let file_name: &str = &matches.free[0];
    let options = CsplitOptions::new(&matches);
    if file_name == "-" {
        let stdin = io::stdin();
        crash_if_err!(1, csplit_impl::csplit(&options, patterns, stdin.lock()));
    } else {
        let file = return_if_err!(1, File::open(file_name));
        let file_metadata = return_if_err!(1, file.metadata());
        if !file_metadata.is_file() {
            crash!(1, "'{}' is not a regular file", file_name);
        }
        crash_if_err!(1, csplit_impl::csplit(&options, patterns, BufReader::new(file)));
    };
    0
}
