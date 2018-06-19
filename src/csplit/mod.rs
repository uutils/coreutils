#![crate_name = "uu_csplit"]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate uucore;
extern crate getopts;
extern crate regex;
use getopts::Matches;
use regex::Regex;
use std::fs::File;

pub mod csplit_impl;

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

/// The definition of a pattern to match on a line.
#[derive(Debug)]
pub enum Pattern {
    /// Copy the file's content to a split up to, not including, the given line number. The number
    /// of times the pattern is executed is detailed in [`ExecutePattern`].
    UpToLine(usize, ExecutePattern),
    /// Copy the file's content to a split up to, not including, the line matching the regex. The
    /// integer is an offset relative to the matched line of what to include (if positive) or
    /// to exclude (if negative). The number of times the pattern is executed is detailed in
    /// [`ExecutePattern`].
    UpToMatch(Regex, i32, ExecutePattern),
    /// Skip the file's content up to, not including, the line matching the regex. The integer
    /// is an offset relative to the matched line of what to include (if positive) or to exclude
    /// (if negative). The number of times the pattern is executed is detailed in [`ExecutePattern`].
    SkipToMatch(Regex, i32, ExecutePattern),
}

/// The number of times a pattern can be used.
#[derive(Debug)]
pub enum ExecutePattern {
    /// Execute the pattern as many times as possible
    Always,
    /// Execute the pattern a fixed number of times
    Times(usize),
}

impl Iterator for ExecutePattern {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        if let ExecutePattern::Times(ref mut n) = self {
            if *n == 0 {
                None
            } else {
                *n -= 1;
                Some(())
            }
        } else {
            Some(())
        }
    }
}

/// Errors thrown by the csplit command
#[derive(Debug, Fail)]
enum CsplitError {
    #[fail(display = "invalid pattern: {}", _0)]
    InvalidPattern(String),
}

/// Parses the definitions of patterns given on the command line into a list of [`Pattern`]s.
///
/// # Errors
///
/// If a pattern is incorrect, a [`CsplitError::InvalidPattern`] error is returned, which may be
/// due to, e.g.,:
/// - an invalid regular expression;
/// - an invalid number for, e.g., the offset.
fn get_patterns(args: &[String]) -> Result<Vec<Pattern>, CsplitError> {
    let mut patterns = Vec::with_capacity(args.len());
    let to_match_reg =
        Regex::new(r"^(/(?P<UPTO>.+)/|%(?P<SKIPTO>.+)%)(?P<OFFSET>[\+-]\d+)?$").unwrap();
    let execute_ntimes_reg = Regex::new(r"^\{(?P<TIMES>\d+)|\*\}$").unwrap();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        // get the number of times a pattern is repeated, which is at least once plus whatever is
        // in the quantifier.
        let execute_ntimes = match iter.peek() {
            None => ExecutePattern::Times(1),
            Some(&next_item) => {
                match execute_ntimes_reg.captures(next_item) {
                    None => ExecutePattern::Times(1),
                    Some(r) => {
                        // skip the next item
                        iter.next();
                        if let Some(times) = r.name("TIMES") {
                            ExecutePattern::Times(times.as_str().parse::<usize>().unwrap() + 1)
                        } else {
                            ExecutePattern::Always
                        }
                    }
                }
            }
        };

        // get the pattern definition
        if let Some(captures) = to_match_reg.captures(arg) {
            let offset = match captures.name("OFFSET") {
                None => 0,
                Some(m) => m.as_str().parse().unwrap(),
            };
            if let Some(up_to_match) = captures.name("UPTO") {
                let pattern = match Regex::new(up_to_match.as_str()) {
                    Err(_) => {
                        return Err(CsplitError::InvalidPattern(arg.to_string()));
                    }
                    Ok(reg) => reg,
                };
                patterns.push(Pattern::UpToMatch(pattern, offset, execute_ntimes));
            } else if let Some(skip_to_match) = captures.name("SKIPTO") {
                let pattern = match Regex::new(skip_to_match.as_str()) {
                    Err(_) => {
                        return Err(CsplitError::InvalidPattern(arg.to_string()));
                    }
                    Ok(reg) => reg,
                };
                patterns.push(Pattern::SkipToMatch(pattern, offset, execute_ntimes));
            }
        } else if let Some(line_number) = arg.parse::<usize>().ok() {
            patterns.push(Pattern::UpToLine(line_number, execute_ntimes));
        } else {
            return Err(CsplitError::InvalidPattern(arg.to_string()));
        }
    }
    Ok(patterns)
}

/// Command line options for csplit.
#[derive(Debug)]
pub struct CsplitOptions {
    prefix: String,
    suffix_format: String,
    keep_files: bool,
    n_digits: u8,
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
        let n_digits = match matches.opt_str(DIGITS_OPT) {
            None => 2,
            Some(opt) => match opt.parse::<u8>() {
                Ok(digits) => digits,
                Err(_) => crash!(1, "invalid number: '{}'", opt),
            },
        };

        CsplitOptions {
            prefix: matches.opt_str(PREFIX_OPT).unwrap_or("xx".to_string()),
            suffix_format: matches
                .opt_str(SUFFIX_FORMAT_OPT)
                .unwrap_or("{:02}".to_string()),
            n_digits,
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
    // get the file to split
    let file_name: &str = &matches.free[0];
    let file = return_if_err!(1, File::open(file_name));
    let file_metadata = return_if_err!(1, file.metadata());
    if !file_metadata.is_file() {
        show_error!("'{}' is not a regular file", file_name);
        exit!(1);
    }
    // get the patterns to split on
    let patterns = return_if_err!(1, get_patterns(&matches.free[1..]));
    return return_if_err!(
        1,
        csplit_impl::csplit(CsplitOptions::new(&matches), patterns, file,)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_pattern() {
        let input = vec!["bad".to_string()];
        assert!(get_patterns(input.as_slice()).is_err());
    }

    #[test]
    fn up_to_line_pattern() {
        let input: Vec<String> = vec!["42", "24", "{*}", "50", "{4}"]
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 3);
        match patterns.get(0) {
            Some(Pattern::UpToLine(42, ExecutePattern::Times(1))) => (),
            _ => assert!(false),
        };
        match patterns.get(1) {
            Some(Pattern::UpToLine(24, ExecutePattern::Always)) => (),
            _ => assert!(false),
        };
        match patterns.get(2) {
            Some(Pattern::UpToLine(50, ExecutePattern::Times(5))) => (),
            _ => assert!(false),
        };
    }

    #[test]
    fn up_to_match_pattern() {
        let input: Vec<String> = vec![
            "/test1.*end$/",
            "/test2.*end$/",
            "{*}",
            "/test3.*end$/",
            "{4}",
            "/test4.*end$/+3",
            "/test5.*end$/-3",
        ].into_iter()
            .map(|v| v.to_string())
            .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 5);
        match patterns.get(0) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test1.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(1) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Always)) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test2.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(2) {
            Some(Pattern::UpToMatch(reg, 0, ExecutePattern::Times(5))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test3.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(3) {
            Some(Pattern::UpToMatch(reg, 3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test4.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(4) {
            Some(Pattern::UpToMatch(reg, -3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test5.*end$");
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn skip_to_match_pattern() {
        let input: Vec<String> = vec![
            "%test1.*end$%",
            "%test2.*end$%",
            "{*}",
            "%test3.*end$%",
            "{4}",
            "%test4.*end$%+3",
            "%test5.*end$%-3",
        ].into_iter()
            .map(|v| v.to_string())
            .collect();
        let patterns = get_patterns(input.as_slice()).unwrap();
        assert_eq!(patterns.len(), 5);
        match patterns.get(0) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test1.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(1) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Always)) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test2.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(2) {
            Some(Pattern::SkipToMatch(reg, 0, ExecutePattern::Times(5))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test3.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(3) {
            Some(Pattern::SkipToMatch(reg, 3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test4.*end$");
            }
            _ => assert!(false),
        };
        match patterns.get(4) {
            Some(Pattern::SkipToMatch(reg, -3, ExecutePattern::Times(1))) => {
                let parsed_reg = format!("{}", reg);
                assert_eq!(parsed_reg, "test5.*end$");
            }
            _ => assert!(false),
        };
    }
}
