// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim

use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Stdin};
use std::path::Path;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, help_about, help_usage};

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};

const ABOUT: &str = help_about!("comm.md");
const USAGE: &str = help_usage!("comm.md");

mod options {
    pub const COLUMN_1: &str = "1";
    pub const COLUMN_2: &str = "2";
    pub const COLUMN_3: &str = "3";
    pub const DELIMITER: &str = "output-delimiter";
    pub const DELIMITER_DEFAULT: &str = "\t";
    pub const FILE_1: &str = "FILE1";
    pub const FILE_2: &str = "FILE2";
    pub const TOTAL: &str = "total";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
}

enum Input {
    Stdin(Stdin),
    FileIn(BufReader<File>),
}

struct LineReader {
    line_ending: LineEnding,
    input: Input,
}

impl LineReader {
    fn new(input: Input, line_ending: LineEnding) -> Self {
        Self { input, line_ending }
    }

    fn read_line(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let line_ending = self.line_ending.into();

        let result = match &mut self.input {
            Input::Stdin(r) => r.lock().read_until(line_ending, buf),
            Input::FileIn(r) => r.read_until(line_ending, buf),
        };

        if !buf.ends_with(&[line_ending]) {
            buf.push(line_ending);
        }

        result
    }
}

fn comm(a: &mut LineReader, b: &mut LineReader, delim: &str, opts: &ArgMatches) {
    let width_col_1 = usize::from(!opts.get_flag(options::COLUMN_1));
    let width_col_2 = usize::from(!opts.get_flag(options::COLUMN_2));

    let delim_col_2 = delim.repeat(width_col_1);
    let delim_col_3 = delim.repeat(width_col_1 + width_col_2);

    let ra = &mut Vec::new();
    let mut na = a.read_line(ra);
    let rb = &mut Vec::new();
    let mut nb = b.read_line(rb);

    let mut total_col_1 = 0;
    let mut total_col_2 = 0;
    let mut total_col_3 = 0;

    while na.is_ok() || nb.is_ok() {
        let ord = match (na.is_ok(), nb.is_ok()) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (true, true) => match (&na, &nb) {
                (&Ok(0), &Ok(0)) => break,
                (&Ok(0), _) => Ordering::Greater,
                (_, &Ok(0)) => Ordering::Less,
                _ => ra.cmp(&rb),
            },
            _ => unreachable!(),
        };

        match ord {
            Ordering::Less => {
                if !opts.get_flag(options::COLUMN_1) {
                    print!("{}", String::from_utf8_lossy(ra));
                }
                ra.clear();
                na = a.read_line(ra);
                total_col_1 += 1;
            }
            Ordering::Greater => {
                if !opts.get_flag(options::COLUMN_2) {
                    print!("{delim_col_2}{}", String::from_utf8_lossy(rb));
                }
                rb.clear();
                nb = b.read_line(rb);
                total_col_2 += 1;
            }
            Ordering::Equal => {
                if !opts.get_flag(options::COLUMN_3) {
                    print!("{delim_col_3}{}", String::from_utf8_lossy(ra));
                }
                ra.clear();
                rb.clear();
                na = a.read_line(ra);
                nb = b.read_line(rb);
                total_col_3 += 1;
            }
        }
    }

    if opts.get_flag(options::TOTAL) {
        let line_ending = LineEnding::from_zero_flag(opts.get_flag(options::ZERO_TERMINATED));
        print!("{total_col_1}{delim}{total_col_2}{delim}{total_col_3}{delim}total{line_ending}");
    }
}

fn open_file(name: &str, line_ending: LineEnding) -> io::Result<LineReader> {
    if name == "-" {
        Ok(LineReader::new(Input::Stdin(stdin()), line_ending))
    } else {
        let f = File::open(Path::new(name))?;
        Ok(LineReader::new(
            Input::FileIn(BufReader::new(f)),
            line_ending,
        ))
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));
    let filename1 = matches.get_one::<String>(options::FILE_1).unwrap();
    let filename2 = matches.get_one::<String>(options::FILE_2).unwrap();
    let mut f1 = open_file(filename1, line_ending).map_err_context(|| filename1.to_string())?;
    let mut f2 = open_file(filename2, line_ending).map_err_context(|| filename2.to_string())?;

    // Due to default_value(), there must be at least one value here, thus unwrap() must not panic.
    let all_delimiters = matches
        .get_many::<String>(options::DELIMITER)
        .unwrap()
        .map(String::from)
        .collect::<Vec<_>>();
    for delim in &all_delimiters[1..] {
        // Note that this check is very different from ".conflicts_with_self(true).action(ArgAction::Set)",
        // as this accepts duplicate *identical* arguments.
        if delim != &all_delimiters[0] {
            // Note: This intentionally deviate from the GNU error message by inserting the word "conflicting".
            return Err(USimpleError::new(
                1,
                "multiple conflicting output delimiters specified",
            ));
        }
    }
    let delim = match &*all_delimiters[0] {
        "" => "\0",
        delim => delim,
    };
    comm(&mut f1, &mut f2, delim, &matches);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::COLUMN_1)
                .short('1')
                .help("suppress column 1 (lines unique to FILE1)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_2)
                .short('2')
                .help("suppress column 2 (lines unique to FILE2)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_3)
                .short('3')
                .help("suppress column 3 (lines that appear in both files)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .help("separate columns with STR")
                .value_name("STR")
                .default_value(options::DELIMITER_DEFAULT)
                .allow_hyphen_values(true)
                .action(ArgAction::Append)
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .long(options::ZERO_TERMINATED)
                .short('z')
                .overrides_with(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_1)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::FILE_2)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long(options::TOTAL)
                .help("output a summary")
                .action(ArgAction::SetTrue),
        )
}
