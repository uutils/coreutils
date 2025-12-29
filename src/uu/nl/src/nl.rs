// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, set_exit_code};
use uucore::{format_usage, show_error, translate};

mod helper;

// Settings store options used by nl to produce its output.
pub struct Settings {
    // The variables corresponding to the options -h, -b, and -f.
    header_numbering: NumberingStyle,
    body_numbering: NumberingStyle,
    footer_numbering: NumberingStyle,
    // The variable corresponding to -d
    section_delimiter: OsString,
    // The variables corresponding to the options -v, -i, -l, -w.
    starting_line_number: i64,
    line_increment: i64,
    join_blank_lines: u64,
    number_width: usize, // Used with String::from_char, hence usize.
    // The format of the number and the (default value for)
    // renumbering each page.
    number_format: NumberFormat,
    renumber: bool,
    // The string appended to each line number output.
    number_separator: OsString,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            header_numbering: NumberingStyle::None,
            body_numbering: NumberingStyle::NonEmpty,
            footer_numbering: NumberingStyle::None,
            section_delimiter: OsString::from("\\:"),
            starting_line_number: 1,
            line_increment: 1,
            join_blank_lines: 1,
            number_width: 6,
            number_format: NumberFormat::Right,
            renumber: true,
            number_separator: OsString::from("\t"),
        }
    }
}

struct Stats {
    line_number: Option<i64>,
    consecutive_empty_lines: u64,
}

impl Stats {
    fn new(starting_line_number: i64) -> Self {
        Self {
            line_number: Some(starting_line_number),
            consecutive_empty_lines: 0,
        }
    }
}

// NumberingStyle stores which lines are to be numbered.
// The possible options are:
// 1. Number all lines
// 2. Number only nonempty lines
// 3. Don't number any lines at all
// 4. Number all lines that match a basic regular expression.
enum NumberingStyle {
    All,
    NonEmpty,
    None,
    Regex(Box<regex::bytes::Regex>),
}

impl TryFrom<&str> for NumberingStyle {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "a" => Ok(Self::All),
            "t" => Ok(Self::NonEmpty),
            "n" => Ok(Self::None),
            _ if s.starts_with('p') => match regex::bytes::Regex::new(&s[1..]) {
                Ok(re) => Ok(Self::Regex(Box::new(re))),
                Err(_) => Err(translate!("nl-error-invalid-regex")),
            },
            _ => Err(translate!("nl-error-invalid-numbering-style", "style" => s)),
        }
    }
}

// NumberFormat specifies how line numbers are output within their allocated
// space. They are justified to the left or right, in the latter case with
// the option of having all unused space to its left turned into leading zeroes.
#[derive(Default)]
enum NumberFormat {
    Left,
    #[default]
    Right,
    RightZero,
}

impl<T: AsRef<str>> From<T> for NumberFormat {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "ln" => Self::Left,
            "rn" => Self::Right,
            "rz" => Self::RightZero,
            _ => unreachable!("Should have been caught by clap"),
        }
    }
}

impl NumberFormat {
    /// Turns a line number into a `String` with at least `min_width` chars,
    /// formatted according to the `NumberFormat`s variant.
    fn format(&self, number: i64, min_width: usize) -> String {
        match self {
            Self::Left => format!("{number:<min_width$}"),
            Self::Right => format!("{number:>min_width$}"),
            Self::RightZero if number < 0 => format!("-{0:0>1$}", number.abs(), min_width - 1),
            Self::RightZero => format!("{number:0>min_width$}"),
        }
    }
}

enum SectionDelimiter {
    Header,
    Body,
    Footer,
}

impl SectionDelimiter {
    /// A valid section delimiter contains the pattern one to three times,
    /// and nothing else.
    fn parse(bytes: &[u8], pattern: &OsStr) -> Option<Self> {
        let pattern = pattern.as_encoded_bytes();

        if bytes.is_empty() || pattern.is_empty() || bytes.len() % pattern.len() != 0 {
            return None;
        }

        let count = bytes.len() / pattern.len();
        if !(1..=3).contains(&count) {
            return None;
        }

        if bytes
            .chunks_exact(pattern.len())
            .all(|chunk| chunk == pattern)
        {
            match count {
                1 => Some(Self::Footer),
                2 => Some(Self::Body),
                3 => Some(Self::Header),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }
}

pub mod options {
    pub const HELP: &str = "help";
    pub const FILE: &str = "file";
    pub const BODY_NUMBERING: &str = "body-numbering";
    pub const SECTION_DELIMITER: &str = "section-delimiter";
    pub const FOOTER_NUMBERING: &str = "footer-numbering";
    pub const HEADER_NUMBERING: &str = "header-numbering";
    pub const LINE_INCREMENT: &str = "line-increment";
    pub const JOIN_BLANK_LINES: &str = "join-blank-lines";
    pub const NUMBER_FORMAT: &str = "number-format";
    pub const NO_RENUMBER: &str = "no-renumber";
    pub const NUMBER_SEPARATOR: &str = "number-separator";
    pub const STARTING_LINE_NUMBER: &str = "starting-line-number";
    pub const NUMBER_WIDTH: &str = "number-width";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mut settings = Settings::default();

    // Update the settings from the command line options, and terminate the
    // program if some options could not successfully be parsed.
    let parse_errors = helper::parse_options(&mut settings, &matches);
    if !parse_errors.is_empty() {
        return Err(USimpleError::new(
            1,
            format!(
                "{}\n{}",
                translate!("nl-error-invalid-arguments"),
                parse_errors.join("\n")
            ),
        ));
    }

    let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec![OsString::from("-")],
    };

    let mut stats = Stats::new(settings.starting_line_number);

    for file in &files {
        if file == "-" {
            let mut buffer = BufReader::new(stdin());
            nl(&mut buffer, &mut stats, &settings)?;
        } else {
            let path = Path::new(file);

            if path.is_dir() {
                show_error!(
                    "{}",
                    translate!("nl-error-is-directory", "path" => path.maybe_quote())
                );
                set_exit_code(1);
            } else {
                let reader = File::open(path).map_err_context(|| file.maybe_quote().to_string())?;
                let mut buffer = BufReader::new(reader);
                nl(&mut buffer, &mut stats, &settings)?;
            }
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(translate!("nl-about"))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("nl-usage")))
        .after_help(translate!("nl-after-help"))
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("nl-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::BODY_NUMBERING)
                .short('b')
                .long(options::BODY_NUMBERING)
                .help(translate!("nl-help-body-numbering"))
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::SECTION_DELIMITER)
                .short('d')
                .long(options::SECTION_DELIMITER)
                .help(translate!("nl-help-section-delimiter"))
                .value_parser(clap::value_parser!(OsString))
                .value_name("CC"),
        )
        .arg(
            Arg::new(options::FOOTER_NUMBERING)
                .short('f')
                .long(options::FOOTER_NUMBERING)
                .help(translate!("nl-help-footer-numbering"))
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::HEADER_NUMBERING)
                .short('h')
                .long(options::HEADER_NUMBERING)
                .help(translate!("nl-help-header-numbering"))
                .value_name("STYLE"),
        )
        .arg(
            Arg::new(options::LINE_INCREMENT)
                .short('i')
                .long(options::LINE_INCREMENT)
                .help(translate!("nl-help-line-increment"))
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(i64)),
        )
        .arg(
            Arg::new(options::JOIN_BLANK_LINES)
                .short('l')
                .long(options::JOIN_BLANK_LINES)
                .help(translate!("nl-help-join-blank-lines"))
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new(options::NUMBER_FORMAT)
                .short('n')
                .long(options::NUMBER_FORMAT)
                .help(translate!("nl-help-number-format"))
                .value_name("FORMAT")
                .value_parser(["ln", "rn", "rz"]),
        )
        .arg(
            Arg::new(options::NO_RENUMBER)
                .short('p')
                .long(options::NO_RENUMBER)
                .help(translate!("nl-help-no-renumber"))
                .action(ArgAction::SetFalse),
        )
        .arg(
            Arg::new(options::NUMBER_SEPARATOR)
                .short('s')
                .long(options::NUMBER_SEPARATOR)
                .help(translate!("nl-help-number-separator"))
                .value_parser(clap::value_parser!(OsString))
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::STARTING_LINE_NUMBER)
                .short('v')
                .long(options::STARTING_LINE_NUMBER)
                .help(translate!("nl-help-starting-line-number"))
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(i64)),
        )
        .arg(
            Arg::new(options::NUMBER_WIDTH)
                .short('w')
                .long(options::NUMBER_WIDTH)
                .help(translate!("nl-help-number-width"))
                .value_name("NUMBER")
                .value_parser(clap::value_parser!(usize)),
        )
}

/// Helper to write: prefix bytes + line bytes + newline
fn write_line(writer: &mut impl Write, prefix: &[u8], line: &[u8]) -> std::io::Result<()> {
    writer.write_all(prefix)?;
    writer.write_all(line)?;
    writeln!(writer)
}

/// `nl` implements the main functionality for an individual buffer.
fn nl<T: Read>(reader: &mut BufReader<T>, stats: &mut Stats, settings: &Settings) -> UResult<()> {
    let mut writer = BufWriter::new(stdout());
    let mut current_numbering_style = &settings.body_numbering;
    let mut line = Vec::new();

    loop {
        line.clear();
        // reads up to and including b'\n'; returns 0 on EOF
        let n = reader
            .read_until(b'\n', &mut line)
            .map_err_context(|| translate!("nl-error-could-not-read-line"))?;
        if n == 0 {
            break;
        }

        if line.last().copied() == Some(b'\n') {
            line.pop();
        }

        if line.is_empty() {
            stats.consecutive_empty_lines += 1;
        } else {
            stats.consecutive_empty_lines = 0;
        }

        let new_numbering_style = match SectionDelimiter::parse(&line, &settings.section_delimiter)
        {
            Some(SectionDelimiter::Header) => Some(&settings.header_numbering),
            Some(SectionDelimiter::Body) => Some(&settings.body_numbering),
            Some(SectionDelimiter::Footer) => Some(&settings.footer_numbering),
            None => None,
        };

        if let Some(new_style) = new_numbering_style {
            current_numbering_style = new_style;
            if settings.renumber {
                stats.line_number = Some(settings.starting_line_number);
            }
            writeln!(writer).map_err_context(|| translate!("nl-error-could-not-write"))?;
        } else {
            let is_line_numbered = match current_numbering_style {
                // consider $join_blank_lines consecutive empty lines to be one logical line
                // for numbering, and only number the last one
                NumberingStyle::All
                    if line.is_empty()
                        && settings.join_blank_lines > 0
                        && stats.consecutive_empty_lines % settings.join_blank_lines != 0 =>
                {
                    false
                }
                NumberingStyle::All => true,
                NumberingStyle::NonEmpty => !line.is_empty(),
                NumberingStyle::None => false,
                NumberingStyle::Regex(re) => re.is_match(&line),
            };

            if is_line_numbered {
                let Some(line_number) = stats.line_number else {
                    return Err(USimpleError::new(
                        1,
                        translate!("nl-error-line-number-overflow"),
                    ));
                };
                let mut prefix = settings
                    .number_format
                    .format(line_number, settings.number_width)
                    .into_bytes();
                prefix.extend_from_slice(settings.number_separator.as_encoded_bytes());
                write_line(&mut writer, &prefix, &line)
                    .map_err_context(|| translate!("nl-error-could-not-write"))?;
                stats.line_number = line_number.checked_add(settings.line_increment);
            } else {
                let prefix = " ".repeat(settings.number_width + 1);
                write_line(&mut writer, prefix.as_bytes(), &line)
                    .map_err_context(|| translate!("nl-error-could-not-write"))?;
            }
        }
    }
    writer
        .flush()
        .map_err_context(|| translate!("nl-error-could-not-write"))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_format() {
        assert_eq!(NumberFormat::Left.format(12, 1), "12");
        assert_eq!(NumberFormat::Left.format(-12, 1), "-12");
        assert_eq!(NumberFormat::Left.format(12, 4), "12  ");
        assert_eq!(NumberFormat::Left.format(-12, 4), "-12 ");

        assert_eq!(NumberFormat::Right.format(12, 1), "12");
        assert_eq!(NumberFormat::Right.format(-12, 1), "-12");
        assert_eq!(NumberFormat::Right.format(12, 4), "  12");
        assert_eq!(NumberFormat::Right.format(-12, 4), " -12");

        assert_eq!(NumberFormat::RightZero.format(12, 1), "12");
        assert_eq!(NumberFormat::RightZero.format(-12, 1), "-12");
        assert_eq!(NumberFormat::RightZero.format(12, 4), "0012");
        assert_eq!(NumberFormat::RightZero.format(-12, 4), "-012");
    }
}
