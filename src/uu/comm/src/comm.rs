// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim

use clap::ArgMatches;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, BufRead, BufReader, Stdin};
use std::path::Path;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::line_ending::LineEnding;

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
    let width_col_1 = usize::from(!opts.get_flag(crate::options::COLUMN_1));
    let width_col_2 = usize::from(!opts.get_flag(crate::options::COLUMN_2));

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
                if !opts.get_flag(crate::options::COLUMN_1) {
                    print!("{}", String::from_utf8_lossy(ra));
                }
                ra.clear();
                na = a.read_line(ra);
                total_col_1 += 1;
            }
            Ordering::Greater => {
                if !opts.get_flag(crate::options::COLUMN_2) {
                    print!("{delim_col_2}{}", String::from_utf8_lossy(rb));
                }
                rb.clear();
                nb = b.read_line(rb);
                total_col_2 += 1;
            }
            Ordering::Equal => {
                if !opts.get_flag(crate::options::COLUMN_3) {
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

    if opts.get_flag(crate::options::TOTAL) {
        let line_ending =
            LineEnding::from_zero_flag(opts.get_flag(crate::options::ZERO_TERMINATED));
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
    let matches = crate::uu_app().try_get_matches_from(args)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(crate::options::ZERO_TERMINATED));
    let filename1 = matches.get_one::<String>(crate::options::FILE_1).unwrap();
    let filename2 = matches.get_one::<String>(crate::options::FILE_2).unwrap();
    let mut f1 = open_file(filename1, line_ending).map_err_context(|| filename1.to_string())?;
    let mut f2 = open_file(filename2, line_ending).map_err_context(|| filename2.to_string())?;

    // Due to default_value(), there must be at least one value here, thus unwrap() must not panic.
    let all_delimiters = matches
        .get_many::<String>(crate::options::DELIMITER)
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
