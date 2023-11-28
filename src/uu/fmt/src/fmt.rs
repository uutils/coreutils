// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) PSKIP linebreak ostream parasplit tabwidth xanti xprefix

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::io::{BufReader, BufWriter, Read, Stdout};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage, show_warning};

use linebreak::break_lines;
use parasplit::ParagraphStream;

mod linebreak;
mod parasplit;

static ABOUT: &str = help_about!("fmt.md");
const USAGE: &str = help_usage!("fmt.md");
static MAX_WIDTH: usize = 2500;

static OPT_CROWN_MARGIN: &str = "crown-margin";
static OPT_TAGGED_PARAGRAPH: &str = "tagged-paragraph";
static OPT_PRESERVE_HEADERS: &str = "preserve-headers";
static OPT_SPLIT_ONLY: &str = "split-only";
static OPT_UNIFORM_SPACING: &str = "uniform-spacing";
static OPT_PREFIX: &str = "prefix";
static OPT_SKIP_PREFIX: &str = "skip-prefix";
static OPT_EXACT_PREFIX: &str = "exact-prefix";
static OPT_EXACT_SKIP_PREFIX: &str = "exact-skip-prefix";
static OPT_WIDTH: &str = "width";
static OPT_GOAL: &str = "goal";
static OPT_QUICK: &str = "quick";
static OPT_TAB_WIDTH: &str = "tab-width";

static ARG_FILES: &str = "files";

// by default, goal is 93% of width
const DEFAULT_GOAL_TO_WIDTH_RATIO: usize = 93;

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
        let mut tagged = matches.get_flag(OPT_TAGGED_PARAGRAPH);
        let mut crown = matches.get_flag(OPT_CROWN_MARGIN);

        let mail = matches.get_flag(OPT_PRESERVE_HEADERS);
        let uniform = matches.get_flag(OPT_UNIFORM_SPACING);
        let quick = matches.get_flag(OPT_QUICK);
        let split_only = matches.get_flag(OPT_SPLIT_ONLY);

        if crown {
            tagged = false;
        }
        if split_only {
            crown = false;
            tagged = false;
        }

        let xprefix = matches.contains_id(OPT_EXACT_PREFIX);
        let xanti_prefix = matches.contains_id(OPT_SKIP_PREFIX);

        let prefix = matches.get_one::<String>(OPT_PREFIX).map(String::from);
        let anti_prefix = matches.get_one::<String>(OPT_SKIP_PREFIX).map(String::from);

        let width_opt = matches.get_one::<usize>(OPT_WIDTH);
        let goal_opt = matches.get_one::<usize>(OPT_GOAL);
        let (width, goal) = match (width_opt, goal_opt) {
            (Some(&w), Some(&g)) => {
                if g > w {
                    return Err(USimpleError::new(1, "GOAL cannot be greater than WIDTH."));
                }
                (w, g)
            }
            (Some(&w), None) => {
                let g = (w * DEFAULT_GOAL_TO_WIDTH_RATIO / 100).min(w - 3);
                (w, g)
            }
            (None, Some(&g)) => {
                let w = (g * 100 / DEFAULT_GOAL_TO_WIDTH_RATIO).max(g + 3);
                (w, g)
            }
            (None, None) => (75, 70),
        };

        if width > MAX_WIDTH {
            return Err(USimpleError::new(
                1,
                format!("invalid width: '{}': Numerical result out of range", width),
            ));
        }

        let mut tabwidth = 8;
        if let Some(s) = matches.get_one::<String>(OPT_TAB_WIDTH) {
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

/// Parse the command line arguments and return the list of files and formatting options.
///
/// # Arguments
///
/// * `args` - Command line arguments.
///
/// # Returns
///
/// A tuple containing a vector of file names and a `FmtOptions` struct.
fn parse_arguments(args: impl uucore::Args) -> UResult<(Vec<String>, FmtOptions)> {
    let matches = uu_app().try_get_matches_from(args)?;

    let mut files: Vec<String> = matches
        .get_many::<String>(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let fmt_opts = FmtOptions::from_matches(&matches)?;

    if files.is_empty() {
        files.push("-".to_owned());
    }

    Ok((files, fmt_opts))
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
    let mut fp = match file_name {
        "-" => BufReader::new(Box::new(stdin()) as Box<dyn Read + 'static>),
        _ => match File::open(file_name) {
            Ok(f) => BufReader::new(Box::new(f) as Box<dyn Read + 'static>),
            Err(e) => {
                show_warning!("{}: {}", file_name.maybe_quote(), e);
                return Ok(());
            }
        },
    };

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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (files, fmt_opts) = parse_arguments(args)?;

    let mut ostream = BufWriter::new(stdout());

    for file_name in &files {
        process_file(file_name, &fmt_opts, &mut ostream)?;
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_CROWN_MARGIN)
                .short('c')
                .long(OPT_CROWN_MARGIN)
                .help(
                    "First and second line of paragraph \
                    may have different indentations, in which \
                    case the first line's indentation is preserved, \
                    and each subsequent line's indentation matches the second line.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_TAGGED_PARAGRAPH)
                .short('t')
                .long("tagged-paragraph")
                .help(
                    "Like -c, except that the first and second line of a paragraph *must* \
                    have different indentation or they are treated as separate paragraphs.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PRESERVE_HEADERS)
                .short('m')
                .long("preserve-headers")
                .help(
                    "Attempt to detect and preserve mail headers in the input. \
                    Be careful when combining this flag with -p.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SPLIT_ONLY)
                .short('s')
                .long("split-only")
                .help("Split lines only, do not reflow.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_UNIFORM_SPACING)
                .short('u')
                .long("uniform-spacing")
                .help(
                    "Insert exactly one \
                    space between words, and two between sentences. \
                    Sentence breaks in the input are detected as [?!.] \
                    followed by two spaces or a newline; other punctuation \
                    is not interpreted as a sentence break.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PREFIX)
                .short('p')
                .long("prefix")
                .help(
                    "Reformat only lines \
                    beginning with PREFIX, reattaching PREFIX to reformatted lines. \
                    Unless -x is specified, leading whitespace will be ignored \
                    when matching PREFIX.",
                )
                .value_name("PREFIX"),
        )
        .arg(
            Arg::new(OPT_SKIP_PREFIX)
                .short('P')
                .long("skip-prefix")
                .help(
                    "Do not reformat lines \
                    beginning with PSKIP. Unless -X is specified, leading whitespace \
                    will be ignored when matching PSKIP",
                )
                .value_name("PSKIP"),
        )
        .arg(
            Arg::new(OPT_EXACT_PREFIX)
                .short('x')
                .long("exact-prefix")
                .help(
                    "PREFIX must match at the \
                    beginning of the line with no preceding whitespace.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_EXACT_SKIP_PREFIX)
                .short('X')
                .long("exact-skip-prefix")
                .help(
                    "PSKIP must match at the \
                    beginning of the line with no preceding whitespace.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_WIDTH)
                .short('w')
                .long("width")
                .help("Fill output lines up to a maximum of WIDTH columns, default 75.")
                .value_name("WIDTH")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new(OPT_GOAL)
                .short('g')
                .long("goal")
                .help("Goal width, default of 93% of WIDTH. Must be less than WIDTH.")
                .value_name("GOAL")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new(OPT_QUICK)
                .short('q')
                .long("quick")
                .help(
                    "Break lines more quickly at the \
            expense of a potentially more ragged appearance.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_TAB_WIDTH)
                .short('T')
                .long("tab-width")
                .help(
                    "Treat tabs as TABWIDTH spaces for \
                    determining line length, default 8. Note that this is used only for \
                    calculating line lengths; tabs are preserved in the output.",
                )
                .value_name("TABWIDTH"),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
