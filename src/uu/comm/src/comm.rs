// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim pairable

use std::cmp::Ordering;
use std::fs::{metadata, File};
use std::io::{self, stdin, BufRead, BufReader, Read, Stdin};
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::fs::paths_refer_to_same_file;
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
    pub const CHECK_ORDER: &str = "check-order";
    pub const NO_CHECK_ORDER: &str = "nocheck-order";
}

#[derive(Debug, Clone, Copy)]
enum FileNumber {
    One,
    Two,
}

impl FileNumber {
    fn as_str(&self) -> &'static str {
        match self {
            FileNumber::One => "1",
            FileNumber::Two => "2",
        }
    }
}

struct OrderChecker {
    last_line: Vec<u8>,
    file_num: FileNumber,
    check_order: bool,
    has_error: bool,
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
        Self { line_ending, input }
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

impl OrderChecker {
    fn new(file_num: FileNumber, check_order: bool) -> Self {
        Self {
            last_line: Vec::new(),
            file_num,
            check_order,
            has_error: false,
        }
    }

    fn verify_order(&mut self, current_line: &[u8]) -> bool {
        if self.last_line.is_empty() {
            self.last_line = current_line.to_vec();
            return true;
        }

        let is_ordered = current_line >= &self.last_line;
        if !is_ordered && !self.has_error {
            eprintln!(
                "comm: file {} is not in sorted order",
                self.file_num.as_str()
            );
            self.has_error = true;
        }

        self.last_line = current_line.to_vec();
        is_ordered || !self.check_order
    }
}

// Check if two files are identical by comparing their contents
pub fn are_files_identical(path1: &str, path2: &str) -> io::Result<bool> {
    // First compare file sizes
    let metadata1 = std::fs::metadata(path1)?;
    let metadata2 = std::fs::metadata(path2)?;

    if metadata1.len() != metadata2.len() {
        return Ok(false);
    }

    let file1 = File::open(path1)?;
    let file2 = File::open(path2)?;

    let mut reader1 = BufReader::new(file1);
    let mut reader2 = BufReader::new(file2);

    let mut buffer1 = [0; 8192];
    let mut buffer2 = [0; 8192];

    loop {
        let bytes1 = reader1.read(&mut buffer1)?;
        let bytes2 = reader2.read(&mut buffer2)?;

        if bytes1 != bytes2 {
            return Ok(false);
        }

        if bytes1 == 0 {
            return Ok(true);
        }

        if buffer1[..bytes1] != buffer2[..bytes2] {
            return Ok(false);
        }
    }
}

fn comm(a: &mut LineReader, b: &mut LineReader, delim: &str, opts: &ArgMatches) -> UResult<()> {
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

    let check_order = opts.get_flag(options::CHECK_ORDER);
    let no_check_order = opts.get_flag(options::NO_CHECK_ORDER);

    // Determine if we should perform order checking
    let should_check_order = !no_check_order
        && (check_order
            || if let (Some(file1), Some(file2)) = (
                opts.get_one::<String>(options::FILE_1),
                opts.get_one::<String>(options::FILE_2),
            ) {
                !(paths_refer_to_same_file(file1, file2, true)
                    || are_files_identical(file1, file2).unwrap_or(false))
            } else {
                true
            });

    let mut checker1 = OrderChecker::new(FileNumber::One, check_order);
    let mut checker2 = OrderChecker::new(FileNumber::Two, check_order);
    let mut input_error = false;

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
                if should_check_order && !checker1.verify_order(ra) {
                    break;
                }
                if !opts.get_flag(options::COLUMN_1) {
                    print!("{}", String::from_utf8_lossy(ra));
                }
                ra.clear();
                na = a.read_line(ra);
                total_col_1 += 1;
            }
            Ordering::Greater => {
                if should_check_order && !checker2.verify_order(rb) {
                    break;
                }
                if !opts.get_flag(options::COLUMN_2) {
                    print!("{delim_col_2}{}", String::from_utf8_lossy(rb));
                }
                rb.clear();
                nb = b.read_line(rb);
                total_col_2 += 1;
            }
            Ordering::Equal => {
                if should_check_order && (!checker1.verify_order(ra) || !checker2.verify_order(rb))
                {
                    break;
                }
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

        // Track if we've seen any order errors
        if (checker1.has_error || checker2.has_error) && !input_error && !check_order {
            input_error = true;
        }
    }

    if opts.get_flag(options::TOTAL) {
        let line_ending = LineEnding::from_zero_flag(opts.get_flag(options::ZERO_TERMINATED));
        print!("{total_col_1}{delim}{total_col_2}{delim}{total_col_3}{delim}total{line_ending}");
    }

    if should_check_order && (checker1.has_error || checker2.has_error) {
        // Print the input error message once at the end
        if input_error {
            eprintln!("comm: input is not in sorted order");
        }
        Err(USimpleError::new(1, ""))
    } else {
        Ok(())
    }
}

fn open_file(name: &str, line_ending: LineEnding) -> io::Result<LineReader> {
    if name == "-" {
        Ok(LineReader::new(Input::Stdin(stdin()), line_ending))
    } else {
        if metadata(name)?.is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, "Is a directory"));
        }
        let f = File::open(name)?;
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

    comm(&mut f1, &mut f2, delim, &matches)
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
        .arg(
            Arg::new(options::CHECK_ORDER)
                .long(options::CHECK_ORDER)
                .help("check that the input is correctly sorted, even if all input lines are pairable")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CHECK_ORDER)
                .long(options::NO_CHECK_ORDER)
                .help("do not check that the input is correctly sorted")
                .action(ArgAction::SetTrue)
                .conflicts_with(options::CHECK_ORDER),
        )
}
