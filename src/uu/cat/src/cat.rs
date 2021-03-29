// This file is part of the uutils coreutils package.
//
// (c) Jordi Boggiano <j.boggiano@seld.be>
// (c) Evgeniy Klyuchikov <evgeniy.klyuchikov@gmail.com>
// (c) Joshua S. Miller <jsmiller@uchicago.edu>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) nonprint nonblank nonprinting

#[macro_use]
extern crate quick_error;
#[cfg(unix)]
extern crate unix_socket;
#[macro_use]
extern crate uucore;

// last synced with: cat (GNU coreutils) 8.13
use clap::{App, Arg};
use quick_error::ResultExt;
use std::fs::{metadata, File};
use std::io::{self, stderr, stdin, stdout, BufWriter, Read, Write};
use uucore::fs::is_stdin_interactive;

/// Unix domain socket support
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use unix_socket::UnixStream;

static NAME: &str = "cat";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static SYNTAX: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Concatenate FILE(s), or standard input, to standard output
 With no FILE, or when FILE is -, read standard input.";

#[derive(PartialEq)]
enum NumberingMode {
    None,
    NonEmpty,
    All,
}

quick_error! {
    #[derive(Debug)]
    enum CatError {
        /// Wrapper for io::Error with path context
        Input(err: io::Error, path: String) {
            display("cat: {0}: {1}", path, err)
            context(path: &'a str, err: io::Error) -> (err, path.to_owned())
            cause(err)
        }

        /// Wrapper for io::Error with no context
        Output(err: io::Error) {
            display("cat: {0}", err) from()
            cause(err)
        }

        /// Unknown Filetype  classification
        UnknownFiletype(path: String) {
            display("cat: {0}: unknown filetype", path)
        }

        /// At least one error was encountered in reading or writing
        EncounteredErrors(count: usize) {
            display("cat: encountered {0} errors", count)
        }

        /// Denotes an error caused by trying to `cat` a directory
        IsDirectory(path: String) {
            display("cat: {0}: Is a directory", path)
        }
    }
}

struct OutputOptions {
    /// Line numbering mode
    number: NumberingMode,

    /// Suppress repeated empty output lines
    squeeze_blank: bool,

    /// display TAB characters as `tab`
    show_tabs: bool,

    /// If `show_tabs == true`, this string will be printed in the
    /// place of tabs
    tab: String,

    /// Can be set to show characters other than '\n' a the end of
    /// each line, e.g. $
    end_of_line: String,

    /// use ^ and M- notation, except for LF (\\n) and TAB (\\t)
    show_nonprint: bool,
}

/// Represents an open file handle, stream, or other device
struct InputHandle {
    reader: Box<dyn Read>,
    is_interactive: bool,
}

/// Concrete enum of recognized file types.
///
/// *Note*: `cat`-ing a directory should result in an
/// CatError::IsDirectory
enum InputType {
    Directory,
    File,
    StdIn,
    SymLink,
    #[cfg(unix)]
    BlockDevice,
    #[cfg(unix)]
    CharacterDevice,
    #[cfg(unix)]
    Fifo,
    #[cfg(unix)]
    Socket,
}

type CatResult<T> = Result<T, CatError>;

mod options {
    pub static FILE: &str = "file";
    pub static SHOW_ALL: &str = "show-all";
    pub static NUMBER_NONBLANK: &str = "number-nonblank";
    pub static SHOW_NONPRINTING_ENDS: &str = "e";
    pub static SHOW_ENDS: &str = "show-ends";
    pub static NUMBER: &str = "number";
    pub static SQUEEZE_BLANK: &str = "squeeze-blank";
    pub static SHOW_NONPRINTING_TABS: &str = "t";
    pub static SHOW_TABS: &str = "show-tabs";
    pub static SHOW_NONPRINTING: &str = "show-nonprinting";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let matches = App::new(executable!())
        .name(NAME)
        .version(VERSION)
        .usage(SYNTAX)
        .about(SUMMARY)
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
        .arg(
            Arg::with_name(options::SHOW_ALL)
                .short("A")
                .long(options::SHOW_ALL)
                .help("equivalent to -vET"),
        )
        .arg(
            Arg::with_name(options::NUMBER_NONBLANK)
                .short("b")
                .long(options::NUMBER_NONBLANK)
                .help("number nonempty output lines, overrides -n")
                .overrides_with(options::NUMBER),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING_ENDS)
                .short("e")
                .help("equivalent to -vE"),
        )
        .arg(
            Arg::with_name(options::SHOW_ENDS)
                .short("E")
                .long(options::SHOW_ENDS)
                .help("display $ at end of each line"),
        )
        .arg(
            Arg::with_name(options::NUMBER)
                .short("n")
                .long(options::NUMBER)
                .help("number all output lines"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE_BLANK)
                .short("s")
                .long(options::SQUEEZE_BLANK)
                .help("suppress repeated empty output lines"),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING_TABS)
                .short("t")
                .long(options::SHOW_NONPRINTING_TABS)
                .help("equivalent to -vT"),
        )
        .arg(
            Arg::with_name(options::SHOW_TABS)
                .short("T")
                .long(options::SHOW_TABS)
                .help("display TAB characters at ^I"),
        )
        .arg(
            Arg::with_name(options::SHOW_NONPRINTING)
                .short("v")
                .long(options::SHOW_NONPRINTING)
                .help("use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        )
        .get_matches_from(args);

    let number_mode = if matches.is_present(options::NUMBER_NONBLANK) {
        NumberingMode::NonEmpty
    } else if matches.is_present(options::NUMBER) {
        NumberingMode::All
    } else {
        NumberingMode::None
    };

    let show_nonprint = vec![
        options::SHOW_ALL.to_owned(),
        options::SHOW_NONPRINTING_ENDS.to_owned(),
        options::SHOW_NONPRINTING_TABS.to_owned(),
        options::SHOW_NONPRINTING.to_owned(),
    ]
    .iter()
    .any(|v| matches.is_present(v));

    let show_ends = vec![
        options::SHOW_ENDS.to_owned(),
        options::SHOW_ALL.to_owned(),
        options::SHOW_NONPRINTING_ENDS.to_owned(),
    ]
    .iter()
    .any(|v| matches.is_present(v));

    let show_tabs = vec![
        options::SHOW_ALL.to_owned(),
        options::SHOW_TABS.to_owned(),
        options::SHOW_NONPRINTING_TABS.to_owned(),
    ]
    .iter()
    .any(|v| matches.is_present(v));

    let squeeze_blank = matches.is_present(options::SQUEEZE_BLANK);
    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec!["-".to_owned()],
    };

    let can_write_fast = !(show_tabs
        || show_nonprint
        || show_ends
        || squeeze_blank
        || number_mode != NumberingMode::None);

    let success = if can_write_fast {
        write_fast(files).is_ok()
    } else {
        let tab = if show_tabs { "^I" } else { "\t" }.to_owned();

        let end_of_line = if show_ends { "$\n" } else { "\n" }.to_owned();

        let options = OutputOptions {
            end_of_line,
            number: number_mode,
            show_nonprint,
            show_tabs,
            squeeze_blank,
            tab,
        };

        write_lines(files, &options).is_ok()
    };

    if success {
        0
    } else {
        1
    }
}

/// Classifies the `InputType` of file at `path` if possible
///
/// # Arguments
///
/// * `path` - Path on a file system to classify metadata
fn get_input_type(path: &str) -> CatResult<InputType> {
    if path == "-" {
        return Ok(InputType::StdIn);
    }

    match metadata(path).context(path)?.file_type() {
        #[cfg(unix)]
        ft if ft.is_block_device() => Ok(InputType::BlockDevice),
        #[cfg(unix)]
        ft if ft.is_char_device() => Ok(InputType::CharacterDevice),
        #[cfg(unix)]
        ft if ft.is_fifo() => Ok(InputType::Fifo),
        #[cfg(unix)]
        ft if ft.is_socket() => Ok(InputType::Socket),
        ft if ft.is_dir() => Ok(InputType::Directory),
        ft if ft.is_file() => Ok(InputType::File),
        ft if ft.is_symlink() => Ok(InputType::SymLink),
        _ => Err(CatError::UnknownFiletype(path.to_owned())),
    }
}

/// Returns an InputHandle from which a Reader can be accessed or an
/// error
///
/// # Arguments
///
/// * `path` - `InputHandler` will wrap a reader from this file path
fn open(path: &str) -> CatResult<InputHandle> {
    if path == "-" {
        let stdin = stdin();
        return Ok(InputHandle {
            reader: Box::new(stdin) as Box<dyn Read>,
            is_interactive: is_stdin_interactive(),
        });
    }

    match get_input_type(path)? {
        InputType::Directory => Err(CatError::IsDirectory(path.to_owned())),
        #[cfg(unix)]
        InputType::Socket => {
            let socket = UnixStream::connect(path).context(path)?;
            socket.shutdown(Shutdown::Write).context(path)?;
            Ok(InputHandle {
                reader: Box::new(socket) as Box<dyn Read>,
                is_interactive: false,
            })
        }
        _ => {
            let file = File::open(path).context(path)?;
            Ok(InputHandle {
                reader: Box::new(file) as Box<dyn Read>,
                is_interactive: false,
            })
        }
    }
}

/// Writes files to stdout with no configuration.  This allows a
/// simple memory copy. Returns `Ok(())` if no errors were
/// encountered, or an error with the number of errors encountered.
///
/// # Arguments
///
/// * `files` - There is no short circuit when encountering an error
/// reading a file in this vector
fn write_fast(files: Vec<String>) -> CatResult<()> {
    let mut writer = stdout();
    let mut in_buf = [0; 1024 * 64];
    let mut error_count = 0;

    for file in files {
        match open(&file[..]) {
            Ok(mut handle) => {
                while let Ok(n) = handle.reader.read(&mut in_buf) {
                    if n == 0 {
                        break;
                    }
                    writer.write_all(&in_buf[..n]).context(&file[..])?;
                }
            }
            Err(error) => {
                writeln!(&mut stderr(), "{}", error)?;
                error_count += 1;
            }
        }
    }

    match error_count {
        0 => Ok(()),
        _ => Err(CatError::EncounteredErrors(error_count)),
    }
}

/// State that persists between output of each file
struct OutputState {
    /// The current line number
    line_number: usize,

    /// Whether the output cursor is at the beginning of a new line
    at_line_start: bool,
}

/// Writes files to stdout with `options` as configuration.  Returns
/// `Ok(())` if no errors were encountered, or an error with the
/// number of errors encountered.
///
/// # Arguments
///
/// * `files` - There is no short circuit when encountering an error
/// reading a file in this vector
fn write_lines(files: Vec<String>, options: &OutputOptions) -> CatResult<()> {
    let mut error_count = 0;
    let mut state = OutputState {
        line_number: 1,
        at_line_start: true,
    };

    for file in files {
        if let Err(error) = write_file_lines(&file, options, &mut state) {
            writeln!(&mut stderr(), "{}", error).context(&file[..])?;
            error_count += 1;
        }
    }

    match error_count {
        0 => Ok(()),
        _ => Err(CatError::EncounteredErrors(error_count)),
    }
}

/// Outputs file contents to stdout in a line-by-line fashion,
/// propagating any errors that might occur.
fn write_file_lines(file: &str, options: &OutputOptions, state: &mut OutputState) -> CatResult<()> {
    let mut handle = open(file)?;
    let mut in_buf = [0; 1024 * 31];
    let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
    let mut one_blank_kept = false;

    while let Ok(n) = handle.reader.read(&mut in_buf) {
        if n == 0 {
            break;
        }
        let in_buf = &in_buf[..n];
        let mut pos = 0;
        while pos < n {
            // skip empty line_number enumerating them if needed
            if in_buf[pos] == b'\n' {
                if !state.at_line_start || !options.squeeze_blank || !one_blank_kept {
                    one_blank_kept = true;
                    if state.at_line_start && options.number == NumberingMode::All {
                        write!(&mut writer, "{0:6}\t", state.line_number)?;
                        state.line_number += 1;
                    }
                    writer.write_all(options.end_of_line.as_bytes())?;
                    if handle.is_interactive {
                        writer.flush().context(file)?;
                    }
                }
                state.at_line_start = true;
                pos += 1;
                continue;
            }
            one_blank_kept = false;
            if state.at_line_start && options.number != NumberingMode::None {
                write!(&mut writer, "{0:6}\t", state.line_number)?;
                state.line_number += 1;
            }

            // print to end of line or end of buffer
            let offset = if options.show_nonprint {
                write_nonprint_to_end(&in_buf[pos..], &mut writer, options.tab.as_bytes())
            } else if options.show_tabs {
                write_tab_to_end(&in_buf[pos..], &mut writer)
            } else {
                write_to_end(&in_buf[pos..], &mut writer)
            };
            // end of buffer?
            if offset == 0 {
                state.at_line_start = false;
                break;
            }
            // print suitable end of line
            writer.write_all(options.end_of_line.as_bytes())?;
            if handle.is_interactive {
                writer.flush()?;
            }
            state.at_line_start = true;
            pos += offset;
        }
    }

    Ok(())
}

// write***_to_end methods
// Write all symbols till end of line or end of buffer is reached
// Return the (number of written symbols + 1) or 0 if the end of buffer is reached
fn write_to_end<W: Write>(in_buf: &[u8], writer: &mut W) -> usize {
    match in_buf.iter().position(|c| *c == b'\n') {
        Some(p) => {
            writer.write_all(&in_buf[..p]).unwrap();
            p + 1
        }
        None => {
            writer.write_all(in_buf).unwrap();
            0
        }
    }
}

fn write_tab_to_end<W: Write>(mut in_buf: &[u8], writer: &mut W) -> usize {
    let mut count = 0;
    loop {
        match in_buf.iter().position(|c| *c == b'\n' || *c == b'\t') {
            Some(p) => {
                writer.write_all(&in_buf[..p]).unwrap();
                if in_buf[p] == b'\n' {
                    return count + p + 1;
                } else {
                    writer.write_all(b"^I").unwrap();
                    in_buf = &in_buf[p + 1..];
                    count += p + 1;
                }
            }
            None => {
                writer.write_all(in_buf).unwrap();
                return 0;
            }
        };
    }
}

fn write_nonprint_to_end<W: Write>(in_buf: &[u8], writer: &mut W, tab: &[u8]) -> usize {
    let mut count = 0;

    for byte in in_buf.iter().map(|c| *c) {
        if byte == b'\n' {
            break;
        }
        match byte {
            9 => writer.write_all(tab),
            0..=8 | 10..=31 => writer.write_all(&[b'^', byte + 64]),
            32..=126 => writer.write_all(&[byte]),
            127 => writer.write_all(&[b'^', byte - 64]),
            128..=159 => writer.write_all(&[b'M', b'-', b'^', byte - 64]),
            160..=254 => writer.write_all(&[b'M', b'-', byte - 128]),
            _ => writer.write_all(&[b'M', b'-', b'^', 63]),
        }
        .unwrap();
        count += 1;
    }
    if count != in_buf.len() {
        count + 1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::io::{stdout, BufWriter};

    #[test]
    fn test_write_nonprint_to_end_new_line() {
        let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
        let in_buf = b"\n";
        let tab = b"";
        super::write_nonprint_to_end(in_buf, &mut writer, tab);
        assert_eq!(writer.buffer().len(), 0);
    }

    #[test]
    fn test_write_nonprint_to_end_9() {
        let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
        let in_buf = &[9u8];
        let tab = b"tab";
        super::write_nonprint_to_end(in_buf, &mut writer, tab);
        assert_eq!(writer.buffer(), tab);
    }

    #[test]
    fn test_write_nonprint_to_end_0_to_8() {
        for byte in 0u8..=8u8 {
            let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
            let in_buf = &[byte];
            let tab = b"";
            super::write_nonprint_to_end(in_buf, &mut writer, tab);
            assert_eq!(writer.buffer(), [b'^', byte + 64]);
        }
    }

    #[test]
    fn test_write_nonprint_to_end_10_to_31() {
        for byte in 11u8..=31u8 {
            let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
            let in_buf = &[byte];
            let tab = b"";
            super::write_nonprint_to_end(in_buf, &mut writer, tab);
            assert_eq!(writer.buffer(), [b'^', byte + 64]);
        }
    }
}
