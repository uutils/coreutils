// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) PSKIP linebreak ostream parasplit tabwidth xanti xprefix

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Stdout, Write, stdin, stdout};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::translate;

use uucore::format_usage;

use linebreak::break_lines;
use parasplit::ParagraphStream;
use thiserror::Error;

mod linebreak;
mod parasplit;

#[derive(Debug, Error)]
enum FmtError {
    #[error("{}", translate!("fmt-error-invalid-goal", "goal" => .0.quote()))]
    InvalidGoal(String),
    #[error("{}", translate!("fmt-error-goal-greater-than-width"))]
    GoalGreaterThanWidth,
    #[error("{}", translate!("fmt-error-invalid-width", "width" => .0.quote()))]
    InvalidWidth(String),
    #[error("{}", translate!("fmt-error-width-out-of-range", "width" => .0))]
    WidthOutOfRange(usize),
    #[error("{}", translate!("fmt-error-invalid-tabwidth", "tabwidth" => .0.quote()))]
    InvalidTabWidth(String),
    #[error("{}", translate!("fmt-error-first-option-width", "option" => .0))]
    FirstOptionWidth(char),
    #[error("{}", translate!("fmt-error-read"))]
    ReadError,
    #[error("{}", translate!("fmt-error-invalid-width-malformed", "width" => .0.quote()))]
    InvalidWidthMalformed(String),
}

impl From<FmtError> for Box<dyn uucore::error::UError> {
    fn from(err: FmtError) -> Self {
        USimpleError::new(1, err.to_string())
    }
}

const MAX_WIDTH: usize = 2500;
const DEFAULT_GOAL: usize = 70;
const DEFAULT_WIDTH: usize = 75;
// by default, goal is 93% of width
const DEFAULT_GOAL_TO_WIDTH_RATIO: usize = 93;

mod options {
    pub const CROWN_MARGIN: &str = "crown-margin";
    pub const TAGGED_PARAGRAPH: &str = "tagged-paragraph";
    pub const PRESERVE_HEADERS: &str = "preserve-headers";
    pub const SPLIT_ONLY: &str = "split-only";
    pub const UNIFORM_SPACING: &str = "uniform-spacing";
    pub const PREFIX: &str = "prefix";
    pub const SKIP_PREFIX: &str = "skip-prefix";
    pub const EXACT_PREFIX: &str = "exact-prefix";
    pub const EXACT_SKIP_PREFIX: &str = "exact-skip-prefix";
    pub const WIDTH: &str = "width";
    pub const GOAL: &str = "goal";
    pub const QUICK: &str = "quick";
    pub const TAB_WIDTH: &str = "tab-width";
    pub const FILES_OR_WIDTH: &str = "files";
}

pub type FileOrStdReader = BufReader<Box<dyn Read + 'static>>;

pub struct FmtOptions {
    crown: bool,
    tagged: bool,
    mail: bool,
    split_only: bool,
    prefix: Option<String>,
    xprefix: bool,
    anti_prefix: Option<String>,
    xanti_prefix: bool,
    uniform: bool,
    quick: bool,
    width: usize,
    goal: usize,
    tabwidth: usize,
}

impl FmtOptions {
    fn from_matches(matches: &ArgMatches) -> UResult<Self> {
        let mut tagged = matches.get_flag(options::TAGGED_PARAGRAPH);
        let mut crown = matches.get_flag(options::CROWN_MARGIN);

        let mail = matches.get_flag(options::PRESERVE_HEADERS);
        let uniform = matches.get_flag(options::UNIFORM_SPACING);
        let quick = matches.get_flag(options::QUICK);
        let split_only = matches.get_flag(options::SPLIT_ONLY);

        if crown {
            tagged = false;
        }
        if split_only {
            crown = false;
            tagged = false;
        }

        let xprefix = matches.contains_id(options::EXACT_PREFIX);
        let xanti_prefix = matches.contains_id(options::SKIP_PREFIX);

        let prefix = matches.get_one::<String>(options::PREFIX).map(String::from);
        let anti_prefix = matches
            .get_one::<String>(options::SKIP_PREFIX)
            .map(String::from);

        let width_opt = extract_width(matches)?;
        let goal_opt_str = matches.get_one::<String>(options::GOAL);
        let goal_opt = if let Some(goal_str) = goal_opt_str {
            match goal_str.parse::<usize>() {
                Ok(goal) => Some(goal),
                Err(_) => {
                    return Err(FmtError::InvalidGoal(goal_str.clone()).into());
                }
            }
        } else {
            None
        };

        let (width, goal) = match (width_opt, goal_opt) {
            (Some(w), Some(g)) => {
                if g > w {
                    return Err(FmtError::GoalGreaterThanWidth.into());
                }
                (w, g)
            }
            (Some(0), None) => {
                // Only allow a goal of zero if the width is set to be zero
                (0, 0)
            }
            (Some(w), None) => {
                let g = (w * DEFAULT_GOAL_TO_WIDTH_RATIO / 100).max(1);
                (w, g)
            }
            (None, Some(g)) => {
                if g > DEFAULT_WIDTH {
                    return Err(FmtError::GoalGreaterThanWidth.into());
                }
                let w = (g * 100 / DEFAULT_GOAL_TO_WIDTH_RATIO).max(g + 3);
                (w, g)
            }
            (None, None) => (DEFAULT_WIDTH, DEFAULT_GOAL),
        };
        debug_assert!(
            width >= goal,
            "GOAL {goal} should not be greater than WIDTH {width} when given {width_opt:?} and {goal_opt:?}."
        );

        if width > MAX_WIDTH {
            return Err(FmtError::WidthOutOfRange(width).into());
        }

        let mut tabwidth = 8;
        if let Some(s) = matches.get_one::<String>(options::TAB_WIDTH) {
            tabwidth = match s.parse::<usize>() {
                Ok(t) => t,
                Err(_) => {
                    return Err(FmtError::InvalidTabWidth(s.clone()).into());
                }
            };
        }

        if tabwidth < 1 {
            tabwidth = 1;
        }

        Ok(Self {
            crown,
            tagged,
            mail,
            split_only,
            prefix,
            xprefix,
            anti_prefix,
            xanti_prefix,
            uniform,
            quick,
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
    file_name: &OsString,
    fmt_opts: &FmtOptions,
    ostream: &mut BufWriter<Stdout>,
) -> UResult<()> {
    let mut fp = BufReader::new(if file_name == "-" {
        Box::new(stdin()) as Box<dyn Read + 'static>
    } else {
        let path = Path::new(file_name);
        let f = File::open(path).map_err_context(
            || translate!("fmt-error-cannot-open-for-reading", "file" => path.quote()),
        )?;
        if f.metadata()
            .map_err_context(
                || translate!("fmt-error-cannot-get-metadata", "file" => path.quote()),
            )?
            .is_dir()
        {
            return Err(FmtError::ReadError.into());
        }

        Box::new(f) as Box<dyn Read + 'static>
    });

    let p_stream = ParagraphStream::new(fmt_opts, &mut fp);
    for para_result in p_stream {
        match para_result {
            Err(s) => {
                ostream
                    .write_all(s.as_bytes())
                    .map_err_context(|| translate!("fmt-error-failed-to-write-output"))?;
                ostream
                    .write_all(b"\n")
                    .map_err_context(|| translate!("fmt-error-failed-to-write-output"))?;
            }
            Ok(para) => break_lines(&para, fmt_opts, ostream)
                .map_err_context(|| translate!("fmt-error-failed-to-write-output"))?,
        }
    }

    // flush the output after each file
    ostream
        .flush()
        .map_err_context(|| translate!("fmt-error-failed-to-write-output"))?;

    Ok(())
}

/// Extract the file names from the positional arguments, ignoring any negative width in the first
/// position.
///
/// # Returns
/// A `UResult<()>` with the file names, or an error if one of the file names could not be parsed
/// (e.g., it is given as a negative number not in the first argument and not after a --
fn extract_files(matches: &ArgMatches) -> UResult<Vec<OsString>> {
    let in_first_pos = matches
        .index_of(options::FILES_OR_WIDTH)
        .is_some_and(|x| x == 1);
    let is_neg = |s: &str| s.parse::<isize>().is_ok_and(|w| w < 0);

    let files: UResult<Vec<OsString>> = matches
        .get_many::<OsString>(options::FILES_OR_WIDTH)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, x)| {
            let x_str = x.to_string_lossy();
            if is_neg(&x_str) {
                if in_first_pos && i == 0 {
                    None
                } else {
                    let first_num = x_str
                        .chars()
                        .nth(1)
                        .expect("a negative number should be at least two characters long");
                    Some(Err(FmtError::FirstOptionWidth(first_num).into()))
                }
            } else {
                Some(Ok(x.clone()))
            }
        })
        .collect();

    if files.as_ref().is_ok_and(|f| f.is_empty()) {
        Ok(vec![OsString::from("-")])
    } else {
        files
    }
}

fn extract_width(matches: &ArgMatches) -> UResult<Option<usize>> {
    let width_opt = matches.get_one::<String>(options::WIDTH);
    if let Some(width_str) = width_opt {
        return if let Ok(width) = width_str.parse::<usize>() {
            Ok(Some(width))
        } else {
            Err(FmtError::InvalidWidth(width_str.clone()).into())
        };
    }

    if let Some(1) = matches.index_of(options::FILES_OR_WIDTH) {
        let width_arg = matches
            .get_one::<OsString>(options::FILES_OR_WIDTH)
            .unwrap();
        let width_str = width_arg.to_string_lossy();
        if let Some(num) = width_str.strip_prefix('-') {
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
            return Err(FmtError::InvalidWidthMalformed(
                first_arg.strip_prefix('-').unwrap().to_string(),
            )
            .into());
        }
    }

    let matches = uucore::clap_localization::handle_clap_result(uu_app(), &args)?;

    let files = extract_files(&matches)?;

    let fmt_opts = FmtOptions::from_matches(&matches)?;

    let mut ostream = BufWriter::new(stdout());

    for file_name in &files {
        process_file(file_name, &fmt_opts, &mut ostream)?;
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("fmt-about"))
        .override_usage(format_usage(&translate!("fmt-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::CROWN_MARGIN)
                .short('c')
                .long(options::CROWN_MARGIN)
                .help(translate!("fmt-crown-margin-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TAGGED_PARAGRAPH)
                .short('t')
                .long("tagged-paragraph")
                .help(translate!("fmt-tagged-paragraph-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESERVE_HEADERS)
                .short('m')
                .long("preserve-headers")
                .help(translate!("fmt-preserve-headers-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPLIT_ONLY)
                .short('s')
                .long("split-only")
                .help(translate!("fmt-split-only-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::UNIFORM_SPACING)
                .short('u')
                .long("uniform-spacing")
                .help(translate!("fmt-uniform-spacing-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PREFIX)
                .short('p')
                .long("prefix")
                .help(translate!("fmt-prefix-help"))
                .value_name("PREFIX"),
        )
        .arg(
            Arg::new(options::SKIP_PREFIX)
                .short('P')
                .long("skip-prefix")
                .help(translate!("fmt-skip-prefix-help"))
                .value_name("PSKIP"),
        )
        .arg(
            Arg::new(options::EXACT_PREFIX)
                .short('x')
                .long("exact-prefix")
                .help(translate!("fmt-exact-prefix-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXACT_SKIP_PREFIX)
                .short('X')
                .long("exact-skip-prefix")
                .help(translate!("fmt-exact-skip-prefix-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long("width")
                .help(translate!("fmt-width-help"))
                // We must accept invalid values if they are overridden later. This is not supported by clap, so accept all strings instead.
                .value_name("WIDTH"),
        )
        .arg(
            Arg::new(options::GOAL)
                .short('g')
                .long("goal")
                .help(translate!("fmt-goal-help"))
                // We must accept invalid values if they are overridden later. This is not supported by clap, so accept all strings instead.
                .value_name("GOAL"),
        )
        .arg(
            Arg::new(options::QUICK)
                .short('q')
                .long("quick")
                .help(translate!("fmt-quick-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TAB_WIDTH)
                .short('T')
                .long("tab-width")
                .help(translate!("fmt-tab-width-help"))
                .value_name("TABWIDTH"),
        )
        .arg(
            Arg::new(options::FILES_OR_WIDTH)
                .action(ArgAction::Append)
                .value_name("FILES")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .allow_negative_numbers(true),
        )
}

#[cfg(test)]
mod tests {
    use crate::uu_app;
    use crate::{extract_files, extract_width};

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
