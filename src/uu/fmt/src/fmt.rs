// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) PSKIP linebreak ostream parasplit tabwidth xanti xprefix

use clap::ArgMatches;
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, Read, Stdout, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};

use crate::linebreak::break_lines;
use crate::parasplit::ParagraphStream;

const MAX_WIDTH: usize = 2500;
const DEFAULT_GOAL: usize = 70;
const DEFAULT_WIDTH: usize = 75;
// by default, goal is 93% of width
const DEFAULT_GOAL_TO_WIDTH_RATIO: usize = 93;

pub type FileOrStdReader = BufReader<Box<dyn Read + 'static>>;

pub struct FmtOptions {
    pub(crate) crown: bool,
    pub(crate) tagged: bool,
    pub(crate) mail: bool,
    pub(crate) split_only: bool,
    pub(crate) prefix: Option<String>,
    pub(crate) xprefix: bool,
    pub(crate) anti_prefix: Option<String>,
    pub(crate) xanti_prefix: bool,
    pub(crate) uniform: bool,
    pub(crate) quick: bool,
    pub(crate) width: usize,
    pub(crate) goal: usize,
    pub(crate) tabwidth: usize,
}

impl FmtOptions {
    fn from_matches(matches: &ArgMatches) -> UResult<Self> {
        let mut tagged = matches.get_flag(crate::options::TAGGED_PARAGRAPH);
        let mut crown = matches.get_flag(crate::options::CROWN_MARGIN);

        let mail = matches.get_flag(crate::options::PRESERVE_HEADERS);
        let uniform = matches.get_flag(crate::options::UNIFORM_SPACING);
        let quick = matches.get_flag(crate::options::QUICK);
        let split_only = matches.get_flag(crate::options::SPLIT_ONLY);

        if crown {
            tagged = false;
        }
        if split_only {
            crown = false;
            tagged = false;
        }

        let xprefix = matches.contains_id(crate::options::EXACT_PREFIX);
        let xanti_prefix = matches.contains_id(crate::options::SKIP_PREFIX);

        let prefix = matches
            .get_one::<String>(crate::options::PREFIX)
            .map(String::from);
        let anti_prefix = matches
            .get_one::<String>(crate::options::SKIP_PREFIX)
            .map(String::from);

        let width_opt = extract_width(matches)?;
        let goal_opt_str = matches.get_one::<String>(crate::options::GOAL);
        let goal_opt = if let Some(goal_str) = goal_opt_str {
            match goal_str.parse::<usize>() {
                Ok(goal) => Some(goal),
                Err(_) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid goal: {}", goal_str.quote()),
                    ));
                }
            }
        } else {
            None
        };

        let (width, goal) = match (width_opt, goal_opt) {
            (Some(w), Some(g)) => {
                if g > w {
                    return Err(USimpleError::new(1, "GOAL cannot be greater than WIDTH."));
                }
                (w, g)
            }
            (Some(w), None) => {
                // Only allow a goal of zero if the width is set to be zero
                let g = (w * DEFAULT_GOAL_TO_WIDTH_RATIO / 100).max(if w == 0 { 0 } else { 1 });
                (w, g)
            }
            (None, Some(g)) => {
                if g > DEFAULT_WIDTH {
                    return Err(USimpleError::new(1, "GOAL cannot be greater than WIDTH."));
                }
                let w = (g * 100 / DEFAULT_GOAL_TO_WIDTH_RATIO).max(g + 3);
                (w, g)
            }
            (None, None) => (DEFAULT_WIDTH, DEFAULT_GOAL),
        };
        debug_assert!(width >= goal, "GOAL {goal} should not be greater than WIDTH {width} when given {width_opt:?} and {goal_opt:?}.");

        if width > MAX_WIDTH {
            return Err(USimpleError::new(
                1,
                format!("invalid width: '{}': Numerical result out of range", width),
            ));
        }

        let mut tabwidth = 8;
        if let Some(s) = matches.get_one::<String>(crate::options::TAB_WIDTH) {
            tabwidth = match s.parse::<usize>() {
                Ok(t) => t,
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format!("Invalid TABWIDTH specification: {}: {}", s.quote(), e),
                    ));
                }
            };
        };

        if tabwidth < 1 {
            tabwidth = 1;
        }

        Ok(Self {
            crown,
            tagged,
            mail,
            uniform,
            quick,
            split_only,
            prefix,
            xprefix,
            anti_prefix,
            xanti_prefix,
            width,
            goal,
            tabwidth,
        })
    }
}

/// Process the content of a file and format it according to the provided options.
///
/// # Arguments
///
/// * `file_name` - The name of the file to process. A value of "-" represents the standard input.
/// * `fmt_opts` - A reference to a `FmtOptions` struct containing the formatting options.
/// * `ostream` - A mutable reference to a `BufWriter` wrapping the standard output.
///
/// # Returns
///
/// A `UResult<()>` indicating success or failure.
fn process_file(
    file_name: &str,
    fmt_opts: &FmtOptions,
    ostream: &mut BufWriter<Stdout>,
) -> UResult<()> {
    let mut fp = BufReader::new(match file_name {
        "-" => Box::new(stdin()) as Box<dyn Read + 'static>,
        _ => {
            let f = File::open(file_name)
                .map_err_context(|| format!("cannot open {} for reading", file_name.quote()))?;
            Box::new(f) as Box<dyn Read + 'static>
        }
    });

    let p_stream = ParagraphStream::new(fmt_opts, &mut fp);
    for para_result in p_stream {
        match para_result {
            Err(s) => {
                ostream
                    .write_all(s.as_bytes())
                    .map_err_context(|| "failed to write output".to_string())?;
                ostream
                    .write_all(b"\n")
                    .map_err_context(|| "failed to write output".to_string())?;
            }
            Ok(para) => break_lines(&para, fmt_opts, ostream)
                .map_err_context(|| "failed to write output".to_string())?,
        }
    }

    // flush the output after each file
    ostream
        .flush()
        .map_err_context(|| "failed to write output".to_string())?;

    Ok(())
}

/// Extract the file names from the positional arguments, ignoring any negative width in the first
/// position.
///
/// # Returns
/// A `UResult<()>` with the file names, or an error if one of the file names could not be parsed
/// (e.g., it is given as a negative number not in the first argument and not after a --
fn extract_files(matches: &ArgMatches) -> UResult<Vec<String>> {
    let in_first_pos = matches
        .index_of(crate::options::FILES_OR_WIDTH)
        .is_some_and(|x| x == 1);
    let is_neg = |s: &str| s.parse::<isize>().is_ok_and(|w| w < 0);

    let files: UResult<Vec<String>> = matches
        .get_many::<String>(crate::options::FILES_OR_WIDTH)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, x)| {
            if is_neg(x) {
                if in_first_pos && i == 0 {
                    None
                } else {
                    let first_num = x.chars().nth(1).expect("a negative number should be at least two characters long");
                    Some(Err(
                        UUsageError::new(1, format!("invalid option -- {}; -WIDTH is recognized only when it is the first\noption; use -w N instead", first_num))
                    ))
                }
            } else {
                Some(Ok(x.clone()))
            }
        })
        .collect();

    if files.as_ref().is_ok_and(|f| f.is_empty()) {
        Ok(vec!["-".into()])
    } else {
        files
    }
}

fn extract_width(matches: &ArgMatches) -> UResult<Option<usize>> {
    let width_opt = matches.get_one::<String>(crate::options::WIDTH);
    if let Some(width_str) = width_opt {
        if let Ok(width) = width_str.parse::<usize>() {
            return Ok(Some(width));
        } else {
            return Err(USimpleError::new(
                1,
                format!("invalid width: {}", width_str.quote()),
            ));
        }
    }

    if let Some(1) = matches.index_of(crate::options::FILES_OR_WIDTH) {
        let width_arg = matches
            .get_one::<String>(crate::options::FILES_OR_WIDTH)
            .unwrap();
        if let Some(num) = width_arg.strip_prefix('-') {
            Ok(num.parse::<usize>().ok())
        } else {
            // will be treated as a file name
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<_> = args.collect();

    // Warn the user if it looks like we're trying to pass a number in the first
    // argument with non-numeric characters
    if let Some(first_arg) = args.get(1) {
        let first_arg = first_arg.to_string_lossy();
        let malformed_number = first_arg.starts_with('-')
            && first_arg.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
            && first_arg.chars().skip(2).any(|c| !c.is_ascii_digit());
        if malformed_number {
            return Err(USimpleError::new(
                1,
                format!(
                    "invalid width: {}",
                    first_arg.strip_prefix('-').unwrap().quote()
                ),
            ));
        }
    }

    let matches = crate::uu_app().try_get_matches_from(&args)?;

    let files = extract_files(&matches)?;

    let fmt_opts = FmtOptions::from_matches(&matches)?;

    let mut ostream = BufWriter::new(stdout());

    for file_name in &files {
        process_file(file_name, &fmt_opts, &mut ostream)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::fmt::{extract_files, extract_width};
    use crate::uu_app;

    #[test]
    fn parse_negative_width() {
        let matches = uu_app()
            .try_get_matches_from(vec!["fmt", "-3", "some-file"])
            .unwrap();

        assert_eq!(extract_files(&matches).unwrap(), vec!["some-file"]);
        assert_eq!(extract_width(&matches).ok(), Some(Some(3)));
    }

    #[test]
    fn parse_width_as_arg() {
        let matches = uu_app()
            .try_get_matches_from(vec!["fmt", "-w3", "some-file"])
            .unwrap();

        assert_eq!(extract_files(&matches).unwrap(), vec!["some-file"]);
        assert_eq!(extract_width(&matches).ok(), Some(Some(3)));
    }

    #[test]
    fn parse_no_args() {
        let matches = uu_app().try_get_matches_from(vec!["fmt"]).unwrap();

        assert_eq!(extract_files(&matches).unwrap(), vec!["-"]);
        assert_eq!(extract_width(&matches).ok(), Some(None));
    }

    #[test]
    fn parse_just_file_name() {
        let matches = uu_app()
            .try_get_matches_from(vec!["fmt", "some-file"])
            .unwrap();

        assert_eq!(extract_files(&matches).unwrap(), vec!["some-file"]);
        assert_eq!(extract_width(&matches).ok(), Some(None));
    }

    #[test]
    fn parse_with_both_widths_positional_first() {
        let matches = uu_app()
            .try_get_matches_from(vec!["fmt", "-10", "-w3", "some-file"])
            .unwrap();

        assert_eq!(extract_files(&matches).unwrap(), vec!["some-file"]);
        assert_eq!(extract_width(&matches).ok(), Some(Some(3)));
    }
}
