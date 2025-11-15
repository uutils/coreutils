// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim mkdelim pairable

use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs::{File, metadata};
use std::io::{self, BufRead, BufReader, Read, StdinLock, stdin};
use std::path::Path;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::format_usage;
use uucore::fs::paths_refer_to_same_file;
use uucore::line_ending::LineEnding;
use uucore::translate;

use clap::{Arg, ArgAction, ArgMatches, Command};

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
            Self::One => "1",
            Self::Two => "2",
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
    Stdin(StdinLock<'static>),
    FileIn(BufReader<File>),
}

impl Input {
    fn stdin() -> Self {
        Self::Stdin(stdin().lock())
    }

    fn from_file(f: File) -> Self {
        Self::FileIn(BufReader::new(f))
    }
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
            Input::Stdin(r) => r.read_until(line_ending, buf),
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

        let is_ordered = *current_line >= *self.last_line;
        if !is_ordered && !self.has_error {
            eprintln!(
                "{}",
                translate!("comm-error-file-not-sorted", "file_num" => self.file_num.as_str())
            );
            self.has_error = true;
        }

        self.last_line = current_line.to_vec();
        is_ordered || !self.check_order
    }
}

// Check if two files are identical by comparing their contents
pub fn are_files_identical(path1: &Path, path2: &Path) -> io::Result<bool> {
    // First compare file sizes
    let metadata1 = metadata(path1)?;
    let metadata2 = metadata(path2)?;

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
        // Read from first file with EINTR retry handling
        // This loop retries the read operation if it's interrupted by signals (e.g., SIGUSR1)
        // instead of failing, which is the POSIX-compliant way to handle interrupted I/O
        let bytes1 = loop {
            match reader1.read(&mut buffer1) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                result => break result?,
            }
        };

        // Read from second file with EINTR retry handling
        // Same retry logic as above for the second file to ensure consistent behavior
        let bytes2 = loop {
            match reader2.read(&mut buffer2) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                result => break result?,
            }
        };

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
                opts.get_one::<OsString>(options::FILE_1),
                opts.get_one::<OsString>(options::FILE_2),
            ) {
                !(paths_refer_to_same_file(file1.as_os_str(), file2.as_os_str(), true)
                    || are_files_identical(Path::new(file1), Path::new(file2)).unwrap_or(false))
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
        print!(
            "{total_col_1}{delim}{total_col_2}{delim}{total_col_3}{delim}{}{line_ending}",
            translate!("comm-total")
        );
    }

    if should_check_order && (checker1.has_error || checker2.has_error) {
        // Print the input error message once at the end
        if input_error {
            eprintln!("{}", translate!("comm-error-input-not-sorted"));
        }
        Err(USimpleError::new(1, ""))
    } else {
        Ok(())
    }
}

fn open_file(name: &OsString, line_ending: LineEnding) -> io::Result<LineReader> {
    if name == "-" {
        Ok(LineReader::new(Input::stdin(), line_ending))
    } else {
        if metadata(name)?.is_dir() {
            return Err(io::Error::other(translate!("comm-error-is-directory")));
        }
        let f = File::open(name)?;
        Ok(LineReader::new(Input::from_file(f), line_ending))
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));
    let filename1 = matches.get_one::<OsString>(options::FILE_1).unwrap();
    let filename2 = matches.get_one::<OsString>(options::FILE_2).unwrap();
    let mut f1 = open_file(filename1, line_ending)
        .map_err_context(|| filename1.to_string_lossy().to_string())?;
    let mut f2 = open_file(filename2, line_ending)
        .map_err_context(|| filename2.to_string_lossy().to_string())?;

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
                translate!("comm-error-multiple-conflicting-delimiters"),
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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("comm-about"))
        .override_usage(format_usage(&translate!("comm-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::COLUMN_1)
                .short('1')
                .help(translate!("comm-help-column-1"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_2)
                .short('2')
                .help(translate!("comm-help-column-2"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_3)
                .short('3')
                .help(translate!("comm-help-column-3"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .help(translate!("comm-help-delimiter"))
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
                .help(translate!("comm-help-zero-terminated"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_1)
                .required(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::FILE_2)
                .required(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long(options::TOTAL)
                .help(translate!("comm-help-total"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHECK_ORDER)
                .long(options::CHECK_ORDER)
                .help(translate!("comm-help-check-order"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CHECK_ORDER)
                .long(options::NO_CHECK_ORDER)
                .help(translate!("comm-help-no-check-order"))
                .action(ArgAction::SetTrue)
                .conflicts_with(options::CHECK_ORDER),
        )
}
