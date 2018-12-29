#![crate_name = "uu_pr"]

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#[cfg(unix)]
extern crate unix_socket;
#[macro_use]
extern crate quick_error;
extern crate chrono;
extern crate getopts;
extern crate itertools;
extern crate uucore;

use chrono::offset::Local;
use chrono::DateTime;
use getopts::{HasArg, Occur};
use getopts::{Matches, Options};
use itertools::structs::KMergeBy;
use itertools::{GroupBy, Itertools};
use quick_error::ResultExt;
use std::convert::From;
use std::fs::{metadata, File, Metadata};
use std::io::{stderr, stdin, stdout, BufRead, BufReader, Lines, Read, Stdin, Stdout, Write};
use std::iter::{Enumerate, Map, SkipWhile, TakeWhile};
use std::num::ParseIntError;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::vec::Vec;

type IOError = std::io::Error;

static NAME: &str = "pr";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static TAB: char = '\t';
static NEW_LINE: &str = "\n";
static LINES_PER_PAGE: usize = 66;
static HEADER_LINES_PER_PAGE: usize = 5;
static TRAILER_LINES_PER_PAGE: usize = 5;
static STRING_HEADER_OPTION: &str = "h";
static DOUBLE_SPACE_OPTION: &str = "d";
static NUMBERING_MODE_OPTION: &str = "n";
static FIRST_LINE_NUMBER_OPTION: &str = "N";
static PAGE_RANGE_OPTION: &str = "pages";
static NO_HEADER_TRAILER_OPTION: &str = "t";
static PAGE_LENGTH_OPTION: &str = "l";
static SUPPRESS_PRINTING_ERROR: &str = "r";
static FORM_FEED_OPTION: &str = "F";
static COLUMN_WIDTH_OPTION: &str = "w";
static ACROSS_OPTION: &str = "a";
static COLUMN_OPTION: &str = "column";
static COLUMN_SEPARATOR_OPTION: &str = "s";
static MERGE_FILES_PRINT: &str = "m";
static OFFSET_SPACES_OPTION: &str = "o";
static FILE_STDIN: &str = "-";
static READ_BUFFER_SIZE: usize = 1024 * 64;
static DEFAULT_COLUMN_WIDTH: usize = 72;
static DEFAULT_COLUMN_SEPARATOR: &char = &TAB;
static BLANK_STRING: &str = "";

struct OutputOptions {
    /// Line numbering mode
    number: Option<NumberingMode>,
    header: String,
    double_space: bool,
    line_separator: String,
    content_line_separator: String,
    last_modified_time: String,
    start_page: usize,
    end_page: Option<usize>,
    display_header: bool,
    display_trailer: bool,
    content_lines_per_page: usize,
    page_separator_char: String,
    column_mode_options: Option<ColumnModeOptions>,
    merge_files_print: Option<usize>,
    offset_spaces: usize,
}

struct FileLine {
    file_id: usize,
    line_number: usize,
    page_number: usize,
    group_key: usize,
    line_content: Result<String, IOError>,
}

impl AsRef<FileLine> for FileLine {
    fn as_ref(&self) -> &FileLine {
        self
    }
}

struct ColumnModeOptions {
    width: usize,
    columns: usize,
    column_separator: String,
    across_mode: bool,
}

impl AsRef<OutputOptions> for OutputOptions {
    fn as_ref(&self) -> &OutputOptions {
        self
    }
}

struct NumberingMode {
    /// Line numbering mode
    width: usize,
    separator: String,
    first_number: usize,
}

impl Default for NumberingMode {
    fn default() -> NumberingMode {
        NumberingMode {
            width: 5,
            separator: TAB.to_string(),
            first_number: 1,
        }
    }
}

impl From<IOError> for PrError {
    fn from(err: IOError) -> Self {
        PrError::EncounteredErrors(err.to_string())
    }
}

quick_error! {
    #[derive(Debug)]
    enum PrError {
        Input(err: IOError, path: String) {
            context(path: &'a str, err: IOError) -> (err, path.to_owned())
            display("pr: Reading from input {0} gave error", path)
            cause(err)
        }

        UnknownFiletype(path: String) {
            display("pr: {0}: unknown filetype", path)
        }

        EncounteredErrors(msg: String) {
            display("pr: {0}", msg)
        }

        IsDirectory(path: String) {
            display("pr: {0}: Is a directory", path)
        }

        IsSocket(path: String) {
            display("pr: cannot open {}, Operation not supported on socket", path)
        }

        NotExists(path: String) {
            display("pr: cannot open {}, No such file or directory", path)
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.opt(
        "",
        PAGE_RANGE_OPTION,
        "Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]",
        "FIRST_PAGE[:LAST_PAGE]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        STRING_HEADER_OPTION,
        "header",
        "Use the string header to replace the file name \
         in the header line.",
        "STRING",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        DOUBLE_SPACE_OPTION,
        "double-space",
        "Produce output that is double spaced. An extra <newline> character is output following every <newline>
           found in the input.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        NUMBERING_MODE_OPTION,
        "number-lines",
        "Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies
           the first width column positions of each text column or each line of -m output.  If char (any nondigit
           character) is given, it is appended to the line number to separate it from whatever follows.  The default
           for char is a <tab>.  Line numbers longer than width columns are truncated.",
        "[char][width]",
        HasArg::Maybe,
        Occur::Optional,
    );

    opts.opt(
        FIRST_LINE_NUMBER_OPTION,
        "first-line-number",
        "start counting with NUMBER at 1st line of first page printed",
        "NUMBER",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        NO_HEADER_TRAILER_OPTION,
        "omit-header",
        "Write neither the five-line identifying header nor the five-line trailer usually supplied for  each  page.  Quit
              writing after the last line of each file without spacing to the end of the page.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        PAGE_LENGTH_OPTION,
        "length",
        "Override the 66-line default and reset the page length to lines.  If lines is not greater than the sum  of  both
              the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the
              -t option were in effect.",
        "lines",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        SUPPRESS_PRINTING_ERROR,
        "no-file-warnings",
        "omit warning when a file cannot be opened",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        FORM_FEED_OPTION,
        "form-feed",
        "Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        "",
        COLUMN_OPTION,
        "Produce multi-column output that is arranged in column columns (the default shall be 1) and is written down each
              column  in  the order in which the text is received from the input file. This option should not be used with -m.
              The options -e and -i shall be assumed for multiple text-column output.  Whether or not text  columns  are  pro‐
              duced  with  identical  vertical  lengths is unspecified, but a text column shall never exceed the length of the
              page (see the -l option). When used with -t, use the minimum number of lines to write the output.",
        "[column]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        COLUMN_WIDTH_OPTION,
        "width",
        "Set  the  width  of the line to width column positions for multiple text-column output only. If the -w option is
              not specified and the -s option is not specified, the default width shall be 72. If the -w option is not  speci‐
              fied and the -s option is specified, the default width shall be 512.",
        "[width]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        ACROSS_OPTION,
        "across",
        "Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order
              (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the
              second line in column 1, and so on).",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        COLUMN_SEPARATOR_OPTION,
        "",
        "Separate text columns by the single character char instead of by the appropriate number of <space>s
           (default for char is the <tab> character).",
        "char",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        MERGE_FILES_PRINT,
        "merge",
        "Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by  a
              file  operand, side by side into text columns of equal fixed widths, in terms of the number of column positions.
              Implementations shall support merging of at least nine file operands.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        OFFSET_SPACES_OPTION,
        "indent",
        "Each  line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset
              shall be zero. The space taken is in addition to the output line width (see the -w option below).",
        "offset",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!("Invalid options\n{}", e),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut files: Vec<String> = matches.free.clone();
    // -n value is optional if -n <path> is given the opts gets confused
    // if -n is used just before file path it might be captured as value of -n
    if matches.opt_str(NUMBERING_MODE_OPTION).is_some() {
        let maybe_a_file_path: String = matches.opt_str(NUMBERING_MODE_OPTION).unwrap();
        let is_file: bool = is_a_file(&maybe_a_file_path);
        if !is_file && files.is_empty() {
            print_error(&matches, PrError::NotExists(maybe_a_file_path));
            return 1;
        } else if is_file {
            files.insert(0, maybe_a_file_path);
        }
    } else if files.is_empty() {
        //For stdin
        files.insert(0, FILE_STDIN.to_owned());
    }

    if matches.opt_present("help") {
        return print_usage(&mut opts, &matches);
    }

    let file_groups: Vec<Vec<String>> = if matches.opt_present(MERGE_FILES_PRINT) {
        vec![files]
    } else {
        files.into_iter().map(|i| vec![i]).collect()
    };

    for file_group in file_groups {
        let result_options: Result<OutputOptions, PrError> = build_options(&matches, &file_group);
        if result_options.is_err() {
            print_error(&matches, result_options.err().unwrap());
            return 1;
        }
        let options: &OutputOptions = &result_options.unwrap();
        let cmd_result: Result<i32, PrError> = if file_group.len() == 1 {
            pr(&file_group.get(0).unwrap(), options)
        } else {
            mpr(&file_group, options)
        };
        let status: i32 = match cmd_result {
            Err(error) => {
                print_error(&matches, error);
                1
            }
            _ => 0,
        };
        if status != 0 {
            return status;
        }
    }
    return 0;
}

fn is_a_file(could_be_file: &String) -> bool {
    could_be_file == FILE_STDIN || File::open(could_be_file).is_ok()
}

fn print_error(matches: &Matches, err: PrError) {
    if !matches.opt_present(SUPPRESS_PRINTING_ERROR) {
        writeln!(&mut stderr(), "{}", err);
    }
}

fn print_usage(opts: &mut Options, matches: &Matches) -> i32 {
    println!("{} {} -- print files", NAME, VERSION);
    println!();
    println!(
        "Usage: {} [+page] [-column] [-adFfmprt] [[-e] [char] [gap]]
        [-L locale] [-h header] [[-i] [char] [gap]]
        [-l lines] [-o offset] [[-s] [char]] [[-n] [char]
        [width]] [-w width] [-] [file ...].",
        NAME
    );
    println!();
    let usage: &str = "The pr utility is a printing and pagination filter
     for text files.  When multiple input files are spec-
     ified, each is read, formatted, and written to stan-
     dard output.  By default, the input is separated
     into 66-line pages, each with

     o   A 5-line header with the page number, date,
         time, and the pathname of the file.

     o   A 5-line trailer consisting of blank lines.

     If standard output is associated with a terminal,
     diagnostic messages are suppressed until the pr
     utility has completed processing.

     When multiple column output is specified, text col-
     umns are of equal width.  By default text columns
     are separated by at least one <blank>.  Input lines
     that do not fit into a text column are truncated.
     Lines are not truncated under single column output.";
    println!("{}", opts.usage(usage));
    if matches.free.is_empty() {
        return 1;
    }
    return 0;
}

fn parse_usize(matches: &Matches, opt: &str) -> Option<Result<usize, PrError>> {
    let from_parse_error_to_pr_error = |value_to_parse: (String, String)| {
        let i = value_to_parse.0;
        let option = value_to_parse.1;
        i.parse::<usize>().map_err(|_e| {
            PrError::EncounteredErrors(format!("invalid {} argument '{}'", option, i))
        })
    };
    matches
        .opt_str(opt)
        .map(|i| (i, format!("-{}", opt)))
        .map(from_parse_error_to_pr_error)
}

fn build_options(matches: &Matches, paths: &Vec<String>) -> Result<OutputOptions, PrError> {
    let invalid_pages_map = |i: String| {
        let unparsed_value: String = matches.opt_str(PAGE_RANGE_OPTION).unwrap();
        i.parse::<usize>().map_err(|_e| {
            PrError::EncounteredErrors(format!("invalid --pages argument '{}'", unparsed_value))
        })
    };

    let is_merge_mode: bool = matches.opt_present(MERGE_FILES_PRINT);

    if is_merge_mode && matches.opt_present(COLUMN_OPTION) {
        let err_msg: String =
            "cannot specify number of columns when printing in parallel".to_string();
        return Err(PrError::EncounteredErrors(err_msg));
    }

    if is_merge_mode && matches.opt_present(ACROSS_OPTION) {
        let err_msg: String =
            "cannot specify both printing across and printing in parallel".to_string();
        return Err(PrError::EncounteredErrors(err_msg));
    }

    let merge_files_print: Option<usize> = if matches.opt_present(MERGE_FILES_PRINT) {
        Some(paths.len())
    } else {
        None
    };

    let header: String = matches
        .opt_str(STRING_HEADER_OPTION)
        .unwrap_or(if is_merge_mode {
            "".to_string()
        } else {
            if paths[0].to_string() == FILE_STDIN {
                "".to_string()
            } else {
                paths[0].to_string()
            }
        });

    let default_first_number: usize = NumberingMode::default().first_number;
    let first_number: usize =
        parse_usize(matches, FIRST_LINE_NUMBER_OPTION).unwrap_or(Ok(default_first_number))?;

    let numbering_options: Option<NumberingMode> = matches
        .opt_str(NUMBERING_MODE_OPTION)
        .map(|i| {
            let parse_result: Result<usize, ParseIntError> = i.parse::<usize>();

            let separator: String = if parse_result.is_err() {
                if is_a_file(&i) {
                    NumberingMode::default().separator
                } else {
                    i[0..1].to_string()
                }
            } else {
                NumberingMode::default().separator
            };

            let width: usize = if parse_result.is_err() {
                if is_a_file(&i) {
                    NumberingMode::default().width
                } else {
                    i[1..]
                        .parse::<usize>()
                        .unwrap_or(NumberingMode::default().width)
                }
            } else {
                parse_result.unwrap()
            };

            NumberingMode {
                width,
                separator,
                first_number,
            }
        })
        .or_else(|| {
            if matches.opt_present(NUMBERING_MODE_OPTION) {
                return Some(NumberingMode::default());
            }
            return None;
        });

    let double_space: bool = matches.opt_present(DOUBLE_SPACE_OPTION);

    let content_line_separator: String = if double_space {
        NEW_LINE.repeat(2)
    } else {
        NEW_LINE.to_string()
    };

    let line_separator: String = NEW_LINE.to_string();

    let last_modified_time: String = if is_merge_mode || paths[0].eq(FILE_STDIN) {
        current_time()
    } else {
        file_last_modified_time(paths.get(0).unwrap())
    };

    let start_page: usize = match matches
        .opt_str(PAGE_RANGE_OPTION)
        .map(|i| {
            let x: Vec<&str> = i.split(":").collect();
            x[0].to_string()
        })
        .map(invalid_pages_map)
    {
        Some(res) => res?,
        _ => 1,
    };

    let end_page: Option<usize> = match matches
        .opt_str(PAGE_RANGE_OPTION)
        .filter(|i: &String| i.contains(":"))
        .map(|i: String| {
            let x: Vec<&str> = i.split(":").collect();
            x[1].to_string()
        })
        .map(invalid_pages_map)
    {
        Some(res) => Some(res?),
        _ => None,
    };

    if end_page.is_some() && start_page > end_page.unwrap() {
        return Err(PrError::EncounteredErrors(format!(
            "invalid --pages argument '{}:{}'",
            start_page,
            end_page.unwrap()
        )));
    }

    let page_length: usize =
        parse_usize(matches, PAGE_LENGTH_OPTION).unwrap_or(Ok(LINES_PER_PAGE))?;

    let page_length_le_ht: bool = page_length < (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE);

    let display_header_and_trailer: bool =
        !(page_length_le_ht) && !matches.opt_present(NO_HEADER_TRAILER_OPTION);

    let content_lines_per_page: usize = if page_length_le_ht {
        page_length
    } else {
        page_length - (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE)
    };

    let page_separator_char: String = matches
        .opt_str(FORM_FEED_OPTION)
        .map(|_i| '\u{000A}'.to_string())
        .unwrap_or(NEW_LINE.to_string());

    let column_width: usize =
        parse_usize(matches, COLUMN_WIDTH_OPTION).unwrap_or(Ok(DEFAULT_COLUMN_WIDTH))?;

    let across_mode: bool = matches.opt_present(ACROSS_OPTION);

    let column_separator: String = matches
        .opt_str(COLUMN_SEPARATOR_OPTION)
        .unwrap_or(DEFAULT_COLUMN_SEPARATOR.to_string());

    let column_mode_options: Option<ColumnModeOptions> = match parse_usize(matches, COLUMN_OPTION) {
        Some(res) => Some(ColumnModeOptions {
            columns: res?,
            width: column_width,
            column_separator,
            across_mode,
        }),
        _ => None,
    };

    let offset_spaces: usize = parse_usize(matches, OFFSET_SPACES_OPTION).unwrap_or(Ok(0))?;

    Ok(OutputOptions {
        number: numbering_options,
        header,
        double_space,
        line_separator,
        content_line_separator,
        last_modified_time,
        start_page,
        end_page,
        display_header: display_header_and_trailer,
        display_trailer: display_header_and_trailer,
        content_lines_per_page,
        page_separator_char,
        column_mode_options,
        merge_files_print,
        offset_spaces,
    })
}

fn open(path: &str) -> Result<Box<Read>, PrError> {
    if path == FILE_STDIN {
        let stdin: Stdin = stdin();
        return Ok(Box::new(stdin) as Box<Read>);
    }

    metadata(path)
        .map(|i: Metadata| {
            let path_string = path.to_string();
            match i.file_type() {
                #[cfg(unix)]
                ft if ft.is_block_device() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_char_device() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_fifo() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_socket() => Err(PrError::IsSocket(path_string)),
                ft if ft.is_dir() => Err(PrError::IsDirectory(path_string)),
                ft if ft.is_file() || ft.is_symlink() => {
                    Ok(Box::new(File::open(path).context(path)?) as Box<Read>)
                }
                _ => Err(PrError::UnknownFiletype(path_string)),
            }
        })
        .unwrap_or(Err(PrError::NotExists(path.to_string())))
}

fn pr(path: &String, options: &OutputOptions) -> Result<i32, PrError> {
    let start_page: &usize = &options.start_page;
    let start_line_number: usize = get_start_line_number(options);
    let last_page: Option<&usize> = options.end_page.as_ref();
    let lines_needed_per_page: usize = lines_to_read_for_page(options);
    let start_line_index_of_start_page = (start_page - 1) * lines_needed_per_page;
    let file_line_groups: GroupBy<
        usize,
        Map<TakeWhile<SkipWhile<Map<Enumerate<Lines<BufReader<Box<Read>>>>, _>, _>, _>, _>,
        _,
    > = BufReader::with_capacity(READ_BUFFER_SIZE, open(path).unwrap())
        .lines()
        .enumerate()
        .map(|i: (usize, Result<String, IOError>)| FileLine {
            file_id: 0,
            line_number: i.0,
            line_content: i.1,
            page_number: 0,
            group_key: 0,
        })
        .skip_while(|file_line: &FileLine| {
            // Skip the initial lines if not in page range
            file_line.line_number < (start_line_index_of_start_page)
        })
        .take_while(|file_line: &FileLine| {
            // Only read the file until provided last page reached
            last_page
                .map(|lp| file_line.line_number < ((*lp) * lines_needed_per_page))
                .unwrap_or(true)
        })
        .map(|file_line: FileLine| {
            let page_number =
                ((file_line.line_number + 1) as f64 / lines_needed_per_page as f64).ceil() as usize;
            FileLine {
                line_number: file_line.line_number + start_line_number,
                page_number,
                group_key: page_number,
                ..file_line
            }
        }) // get display line number with line content
        .group_by(|file_line: &FileLine| file_line.group_key);

    for (page_number, file_line_group) in file_line_groups.into_iter() {
        let mut lines: Vec<FileLine> = Vec::new();
        for file_line in file_line_group {
            if file_line.line_content.is_err() {
                return Err(file_line.line_content.unwrap_err().into());
            }
            lines.push(file_line);
        }
        print_page(&lines, options, &page_number)?;
    }

    return Ok(0);
}

fn mpr(paths: &Vec<String>, options: &OutputOptions) -> Result<i32, PrError> {
    let nfiles = paths.len();

    let lines_needed_per_page: usize = lines_to_read_for_page(options);
    let lines_needed_per_page_f64: f64 = lines_needed_per_page as f64;
    let start_page: &usize = &options.start_page;
    let last_page: Option<&usize> = options.end_page.as_ref();
    let start_line_index_of_start_page = (start_page - 1) * lines_needed_per_page;

    let file_line_groups: GroupBy<
        usize,
        KMergeBy<
            Map<TakeWhile<SkipWhile<Map<Enumerate<Lines<BufReader<Box<Read>>>>, _>, _>, _>, _>,
            _,
        >,
        _,
    > = paths
        .into_iter()
        .enumerate()
        .map(|indexed_path: (usize, &String)| {
            let start_line_number: usize = get_start_line_number(options);
            BufReader::with_capacity(READ_BUFFER_SIZE, open(indexed_path.1).unwrap())
                .lines()
                .enumerate()
                .map(move |i: (usize, Result<String, IOError>)| FileLine {
                    file_id: indexed_path.0,
                    line_number: i.0,
                    line_content: i.1,
                    page_number: 0,
                    group_key: 0,
                })
                .skip_while(move |file_line: &FileLine| {
                    // Skip the initial lines if not in page range
                    file_line.line_number < (start_line_index_of_start_page)
                })
                .take_while(move |file_line: &FileLine| {
                    // Only read the file until provided last page reached
                    last_page
                        .map(|lp| file_line.line_number < ((*lp) * lines_needed_per_page))
                        .unwrap_or(true)
                })
                .map(move |file_line: FileLine| {
                    let page_number = ((file_line.line_number + 2 - start_line_number) as f64
                        / (lines_needed_per_page_f64))
                        .ceil() as usize;
                    FileLine {
                        line_number: file_line.line_number + start_line_number,
                        page_number,
                        group_key: page_number * nfiles + file_line.file_id,
                        ..file_line
                    }
                }) // get display line number with line content
        })
        .kmerge_by(|a: &FileLine, b: &FileLine| {
            if a.group_key == b.group_key {
                a.line_number < b.line_number
            } else {
                a.group_key < b.group_key
            }
        })
        .group_by(|file_line: &FileLine| file_line.group_key);

    let mut lines: Vec<FileLine> = Vec::new();
    let start_page: &usize = &options.start_page;
    let mut page_counter: usize = *start_page;
    for (_key, file_line_group) in file_line_groups.into_iter() {
        for file_line in file_line_group {
            if file_line.line_content.is_err() {
                return Err(file_line.line_content.unwrap_err().into());
            }
            let new_page_number = file_line.page_number;
            if page_counter != new_page_number {
                fill_missing_lines(&mut lines, lines_needed_per_page, &nfiles, page_counter);
                print_page(&lines, options, &page_counter)?;
                lines = Vec::new();
            }
            lines.push(file_line);
            page_counter = new_page_number;
        }
    }

    fill_missing_lines(&mut lines, lines_needed_per_page, &nfiles, page_counter);
    print_page(&lines, options, &page_counter)?;

    return Ok(0);
}

fn fill_missing_lines(
    lines: &mut Vec<FileLine>,
    lines_per_file: usize,
    nfiles: &usize,
    page_number: usize,
) {
    let init_line_number = (page_number - 1) * lines_per_file + 1;
    let mut file_id_counter: usize = 0;
    let mut line_number_counter: usize = init_line_number;
    let mut lines_processed_per_file: usize = 0;
    for mut i in 0..lines_per_file * nfiles {
        let file_id = lines
            .get(i)
            .map(|i: &FileLine| i.file_id)
            .unwrap_or(file_id_counter);
        let line_number = lines.get(i).map(|i: &FileLine| i.line_number).unwrap_or(1);
        if lines_processed_per_file == lines_per_file {
            line_number_counter = init_line_number;
            file_id_counter += 1;
            lines_processed_per_file = 0;
        }

        if file_id != file_id_counter {
            // Insert missing file_ids
            lines.insert(
                i,
                FileLine {
                    file_id: file_id_counter,
                    line_number: line_number_counter,
                    line_content: Ok("".to_string()),
                    page_number,
                    group_key: 0,
                },
            );
            line_number_counter += 1;
        } else if line_number < line_number_counter {
            // Insert missing lines for a file_id
            line_number_counter += 1;
            lines.insert(
                i,
                FileLine {
                    file_id,
                    line_number: line_number_counter,
                    line_content: Ok("".to_string()),
                    page_number,
                    group_key: 0,
                },
            );
        } else {
            line_number_counter = line_number;
        }

        lines_processed_per_file += 1;
    }
}

fn print_page(
    lines: &Vec<FileLine>,
    options: &OutputOptions,
    page: &usize,
) -> Result<usize, IOError> {
    let page_separator = options.page_separator_char.as_bytes();
    let header: Vec<String> = header_content(options, page);
    let trailer_content: Vec<String> = trailer_content(options);

    let out: &mut Stdout = &mut stdout();
    let line_separator = options.line_separator.as_bytes();

    out.lock();
    for x in header {
        out.write(x.as_bytes())?;
        out.write(line_separator)?;
    }

    let lines_written = write_columns(lines, options, out)?;

    for index in 0..trailer_content.len() {
        let x: &String = trailer_content.get(index).unwrap();
        out.write(x.as_bytes())?;
        if index + 1 != trailer_content.len() {
            out.write(line_separator)?;
        }
    }
    out.write(page_separator)?;
    out.flush()?;
    Ok(lines_written)
}

fn write_columns(
    lines: &Vec<FileLine>,
    options: &OutputOptions,
    out: &mut Stdout,
) -> Result<usize, IOError> {
    let line_separator = options.content_line_separator.as_bytes();
    let content_lines_per_page = if options.double_space {
        options.content_lines_per_page / 2
    } else {
        options.content_lines_per_page
    };

    let width: usize = options.number.as_ref().map(|i| i.width).unwrap_or(0);
    let number_separator: String = options
        .number
        .as_ref()
        .map(|i| i.separator.to_string())
        .unwrap_or(NumberingMode::default().separator);

    let blank_line = "".to_string();
    let columns = options.merge_files_print.unwrap_or(get_columns(options));
    let def_sep = DEFAULT_COLUMN_SEPARATOR.to_string();
    let col_sep: &String = options
        .column_mode_options
        .as_ref()
        .map(|i| &i.column_separator)
        .unwrap_or(
            options
                .merge_files_print
                .map(|_k| &def_sep)
                .unwrap_or(&blank_line),
        );

    // TODO simplify
    let col_width: Option<usize> = options
        .column_mode_options
        .as_ref()
        .map(|i| Some(i.width))
        .unwrap_or(
            options
                .merge_files_print
                .map(|_k| Some(DEFAULT_COLUMN_WIDTH))
                .unwrap_or(None),
        );

    let across_mode = options
        .column_mode_options
        .as_ref()
        .map(|i| i.across_mode)
        .unwrap_or(false);

    let offset_spaces: &usize = &options.offset_spaces;

    let mut lines_printed = 0;
    let is_number_mode = options.number.is_some();
    let fetch_indexes: Vec<Vec<usize>> = if across_mode {
        (0..content_lines_per_page)
            .map(|a| (0..columns).map(|i| a * columns + i).collect())
            .collect()
    } else {
        (0..content_lines_per_page)
            .map(|start| {
                (0..columns)
                    .map(|i| start + content_lines_per_page * i)
                    .collect()
            })
            .collect()
    };

    let spaces = " ".repeat(*offset_spaces);

    for fetch_index in fetch_indexes {
        let indexes = fetch_index.len();
        for i in 0..indexes {
            let index: usize = fetch_index[i];
            if lines.get(index).is_none() {
                break;
            }
            let file_line: &FileLine = lines.get(index).unwrap();
            let trimmed_line: String = format!(
                "{}{}",
                spaces,
                get_line_for_printing(
                    file_line,
                    &width,
                    &number_separator,
                    columns,
                    col_width,
                    is_number_mode,
                    &options.merge_files_print,
                    &i,
                )
            );
            out.write(trimmed_line.as_bytes())?;
            if (i + 1) != indexes {
                out.write(col_sep.as_bytes())?;
            }
            lines_printed += 1;
        }
        out.write(line_separator)?;
    }
    Ok(lines_printed)
}

fn get_line_for_printing(
    file_line: &FileLine,
    width: &usize,
    separator: &String,
    columns: usize,
    col_width: Option<usize>,
    is_number_mode: bool,
    merge_files_print: &Option<usize>,
    index: &usize,
) -> String {
    let should_show_line_number_merge_file =
        merge_files_print.is_none() || index == &usize::min_value();
    let should_show_line_number = is_number_mode && should_show_line_number_merge_file;
    let fmtd_line_number: String = if should_show_line_number {
        get_fmtd_line_number(&width, file_line.line_number, &separator)
    } else {
        "".to_string()
    };
    let mut complete_line = format!(
        "{}{}",
        fmtd_line_number,
        file_line.line_content.as_ref().unwrap()
    );

    let tab_count: usize = complete_line.chars().filter(|i| i == &TAB).count();

    let display_length = complete_line.len() + (tab_count * 7);
    // TODO Adjust the width according to -n option
    // TODO actual len of the string vs display len of string because of tabs
    col_width
        .map(|i| {
            let min_width = (i - (columns - 1)) / columns;
            if display_length < min_width {
                for _i in 0..(min_width - display_length) {
                    complete_line.push(' ');
                }
            }

            complete_line.chars().take(min_width).collect()
        })
        .unwrap_or(complete_line)
}

fn get_fmtd_line_number(width: &usize, line_number: usize, separator: &String) -> String {
    let line_str = line_number.to_string();
    if line_str.len() >= *width {
        format!(
            "{:>width$}{}",
            &line_str[line_str.len() - *width..],
            separator,
            width = width
        )
    } else {
        format!("{:>width$}{}", line_str, separator, width = width)
    }
}

fn header_content(options: &OutputOptions, page: &usize) -> Vec<String> {
    if options.display_header {
        let first_line: String = format!(
            "{} {} Page {}",
            options.last_modified_time, options.header, page
        );
        vec![
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
            first_line,
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
        ]
    } else {
        Vec::new()
    }
}

fn file_last_modified_time(path: &str) -> String {
    let file_metadata = metadata(path);
    return file_metadata
        .map(|i| {
            return i
                .modified()
                .map(|x| {
                    let datetime: DateTime<Local> = x.into();
                    datetime.format("%b %d %H:%M %Y").to_string()
                })
                .unwrap_or(String::new());
        })
        .unwrap_or(String::new());
}

fn current_time() -> String {
    let datetime: DateTime<Local> = Local::now();
    datetime.format("%b %d %H:%M %Y").to_string()
}

fn trailer_content(options: &OutputOptions) -> Vec<String> {
    if options.as_ref().display_trailer {
        vec![
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
            BLANK_STRING.to_string(),
        ]
    } else {
        Vec::new()
    }
}

/// Returns starting line number for the file to be printed.
/// If -N is specified the first line number changes otherwise
/// default is 1.
/// # Arguments
/// * `opts` - A reference to OutputOptions
fn get_start_line_number(opts: &OutputOptions) -> usize {
    opts.number.as_ref().map(|i| i.first_number).unwrap_or(1)
}

/// Returns number of lines to read from input for constructing one page of pr output.
/// If double space -d is used lines are halved.
/// If columns --columns is used the lines are multiplied by the value.
/// # Arguments
/// * `opts` - A reference to OutputOptions
fn lines_to_read_for_page(opts: &OutputOptions) -> usize {
    let content_lines_per_page = opts.content_lines_per_page;
    let columns = get_columns(opts);
    if opts.double_space {
        (content_lines_per_page / 2) * columns
    } else {
        content_lines_per_page * columns
    }
}

/// Returns number of columns to output
/// # Arguments
/// * `opts` - A reference to OutputOptions
fn get_columns(opts: &OutputOptions) -> usize {
    opts.column_mode_options
        .as_ref()
        .map(|i| i.columns)
        .unwrap_or(1)
}
