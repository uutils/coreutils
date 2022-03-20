// This file is part of the uutils coreutils package.
//
// (c) Jordi Boggiano <j.boggiano@seld.be>
// (c) Evgeniy Klyuchikov <evgeniy.klyuchikov@gmail.com>
// (c) Joshua S. Miller <jsmiller@uchicago.edu>
// (c) Árni Dagur <arni@dagur.eu>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) nonprint nonblank nonprinting

#[cfg(unix)]
extern crate unix_socket;

// last synced with: cat (GNU coreutils) 8.13
use clap::{crate_version, Arg, Command};
use std::fs::{metadata, File};
use std::io::{self, Read, Write};
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::fs::FileInformation;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// Linux splice support
#[cfg(any(target_os = "linux", target_os = "android"))]
mod splice;

/// Unix domain socket support
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use unix_socket::UnixStream;
use uucore::{format_usage, InvalidEncodingHandling};

static NAME: &str = "cat";
static USAGE: &str = "{} [OPTION]... [FILE]...";
static SUMMARY: &str = "Concatenate FILE(s), or standard input, to standard output
 With no FILE, or when FILE is -, read standard input.";

#[derive(Error, Debug)]
enum CatError {
    /// Wrapper around `io::Error`
    #[error("{0}")]
    Io(#[from] io::Error),
    /// Wrapper around `nix::Error`
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("{0}")]
    Nix(#[from] nix::Error),
    /// Unknown file type; it's not a regular file, socket, etc.
    #[error("unknown filetype: {}", ft_debug)]
    UnknownFiletype {
        /// A debug print of the file type
        ft_debug: String,
    },
    #[error("Is a directory")]
    IsDirectory,
    #[error("input file is output file")]
    OutputIsInput,
}

type CatResult<T> = Result<T, CatError>;

#[derive(PartialEq)]
enum NumberingMode {
    None,
    NonEmpty,
    All,
}

struct OutputOptions {
    /// Line numbering mode
    number: NumberingMode,

    /// Suppress repeated empty output lines
    squeeze_blank: bool,

    /// display TAB characters as `tab`
    show_tabs: bool,

    /// Show end of lines
    show_ends: bool,

    /// use ^ and M- notation, except for LF (\\n) and TAB (\\t)
    show_nonprint: bool,
}

impl OutputOptions {
    fn tab(&self) -> &'static str {
        if self.show_tabs {
            "^I"
        } else {
            "\t"
        }
    }

    fn end_of_line(&self) -> &'static str {
        if self.show_ends {
            "$\n"
        } else {
            "\n"
        }
    }

    /// We can write fast if we can simply copy the contents of the file to
    /// stdout, without augmenting the output with e.g. line numbers.
    fn can_write_fast(&self) -> bool {
        !(self.show_tabs
            || self.show_nonprint
            || self.show_ends
            || self.squeeze_blank
            || self.number != NumberingMode::None)
    }
}

/// State that persists between output of each file. This struct is only used
/// when we can't write fast.
struct OutputState {
    /// The current line number
    line_number: usize,

    /// Whether the output cursor is at the beginning of a new line
    at_line_start: bool,

    /// Whether we skipped a \r, which still needs to be printed
    skipped_carriage_return: bool,

    /// Whether we have already printed a blank line
    one_blank_kept: bool,
}

#[cfg(unix)]
trait FdReadable: Read + AsRawFd {}
#[cfg(not(unix))]
trait FdReadable: Read {}

#[cfg(unix)]
impl<T> FdReadable for T where T: Read + AsRawFd {}
#[cfg(not(unix))]
impl<T> FdReadable for T where T: Read {}

/// Represents an open file handle, stream, or other device
struct InputHandle<R: FdReadable> {
    reader: R,
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

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

    let options = OutputOptions {
        show_ends,
        number: number_mode,
        show_nonprint,
        show_tabs,
        squeeze_blank,
    };
    cat_files(&files, &options)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
        .arg(
            Arg::new(options::SHOW_ALL)
                .short('A')
                .long(options::SHOW_ALL)
                .help("equivalent to -vET"),
        )
        .arg(
            Arg::new(options::NUMBER_NONBLANK)
                .short('b')
                .long(options::NUMBER_NONBLANK)
                .help("number nonempty output lines, overrides -n")
                .overrides_with(options::NUMBER),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_ENDS)
                .short('e')
                .help("equivalent to -vE"),
        )
        .arg(
            Arg::new(options::SHOW_ENDS)
                .short('E')
                .long(options::SHOW_ENDS)
                .help("display $ at end of each line"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .short('n')
                .long(options::NUMBER)
                .help("number all output lines"),
        )
        .arg(
            Arg::new(options::SQUEEZE_BLANK)
                .short('s')
                .long(options::SQUEEZE_BLANK)
                .help("suppress repeated empty output lines"),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_TABS)
                .short('t')
                .long(options::SHOW_NONPRINTING_TABS)
                .help("equivalent to -vT"),
        )
        .arg(
            Arg::new(options::SHOW_TABS)
                .short('T')
                .long(options::SHOW_TABS)
                .help("display TAB characters at ^I"),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING)
                .short('v')
                .long(options::SHOW_NONPRINTING)
                .help("use ^ and M- notation, except for LF (\\n) and TAB (\\t)"),
        )
}

fn cat_handle<R: FdReadable>(
    handle: &mut InputHandle<R>,
    options: &OutputOptions,
    state: &mut OutputState,
) -> CatResult<()> {
    if options.can_write_fast() {
        write_fast(handle)
    } else {
        write_lines(handle, options, state)
    }
}

fn cat_path(
    path: &str,
    options: &OutputOptions,
    state: &mut OutputState,
    out_info: Option<&FileInformation>,
) -> CatResult<()> {
    match get_input_type(path)? {
        InputType::StdIn => {
            let stdin = io::stdin();
            let mut handle = InputHandle {
                reader: stdin,
                is_interactive: atty::is(atty::Stream::Stdin),
            };
            cat_handle(&mut handle, options, state)
        }
        InputType::Directory => Err(CatError::IsDirectory),
        #[cfg(unix)]
        InputType::Socket => {
            let socket = UnixStream::connect(path)?;
            socket.shutdown(Shutdown::Write)?;
            let mut handle = InputHandle {
                reader: socket,
                is_interactive: false,
            };
            cat_handle(&mut handle, options, state)
        }
        _ => {
            let file = File::open(path)?;

            if let Some(out_info) = out_info {
                if out_info.file_size() != 0
                    && FileInformation::from_file(&file).as_ref() == Some(out_info)
                {
                    return Err(CatError::OutputIsInput);
                }
            }

            let mut handle = InputHandle {
                reader: file,
                is_interactive: false,
            };
            cat_handle(&mut handle, options, state)
        }
    }
}

fn cat_files(files: &[String], options: &OutputOptions) -> UResult<()> {
    let out_info = FileInformation::from_file(&std::io::stdout());

    let mut state = OutputState {
        line_number: 1,
        at_line_start: true,
        skipped_carriage_return: false,
        one_blank_kept: false,
    };
    let mut error_messages: Vec<String> = Vec::new();

    for path in files {
        if let Err(err) = cat_path(path, options, &mut state, out_info.as_ref()) {
            error_messages.push(format!("{}: {}", path.maybe_quote(), err));
        }
    }
    if state.skipped_carriage_return {
        print!("\r");
    }
    if error_messages.is_empty() {
        Ok(())
    } else {
        // each next line is expected to display "cat: …"
        let line_joiner = format!("\n{}: ", uucore::util_name());

        Err(uucore::error::USimpleError::new(
            error_messages.len() as i32,
            error_messages.join(&line_joiner),
        ))
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

    let ft = metadata(path)?.file_type();
    match ft {
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
        _ => Err(CatError::UnknownFiletype {
            ft_debug: format!("{:?}", ft),
        }),
    }
}

/// Writes handle to stdout with no configuration. This allows a
/// simple memory copy.
fn write_fast<R: FdReadable>(handle: &mut InputHandle<R>) -> CatResult<()> {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // If we're on Linux or Android, try to use the splice() system call
        // for faster writing. If it works, we're done.
        if !splice::write_fast_using_splice(handle, &stdout_lock)? {
            return Ok(());
        }
    }
    // If we're not on Linux or Android, or the splice() call failed,
    // fall back on slower writing.
    let mut buf = [0; 1024 * 64];
    while let Ok(n) = handle.reader.read(&mut buf) {
        if n == 0 {
            break;
        }
        stdout_lock.write_all(&buf[..n])?;
    }
    Ok(())
}

/// Outputs file contents to stdout in a line-by-line fashion,
/// propagating any errors that might occur.
fn write_lines<R: FdReadable>(
    handle: &mut InputHandle<R>,
    options: &OutputOptions,
    state: &mut OutputState,
) -> CatResult<()> {
    let mut in_buf = [0; 1024 * 31];
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    while let Ok(n) = handle.reader.read(&mut in_buf) {
        if n == 0 {
            break;
        }
        let in_buf = &in_buf[..n];
        let mut pos = 0;
        while pos < n {
            // skip empty line_number enumerating them if needed
            if in_buf[pos] == b'\n' {
                // \r followed by \n is printed as ^M when show_ends is enabled, so that \r\n prints as ^M$
                if state.skipped_carriage_return && options.show_ends {
                    writer.write_all(b"^M")?;
                    state.skipped_carriage_return = false;
                }
                if !state.at_line_start || !options.squeeze_blank || !state.one_blank_kept {
                    state.one_blank_kept = true;
                    if state.at_line_start && options.number == NumberingMode::All {
                        write!(writer, "{0:6}\t", state.line_number)?;
                        state.line_number += 1;
                    }
                    writer.write_all(options.end_of_line().as_bytes())?;
                    if handle.is_interactive {
                        writer.flush()?;
                    }
                }
                state.at_line_start = true;
                pos += 1;
                continue;
            }
            if state.skipped_carriage_return {
                writer.write_all(b"\r")?;
                state.skipped_carriage_return = false;
                state.at_line_start = false;
            }
            state.one_blank_kept = false;
            if state.at_line_start && options.number != NumberingMode::None {
                write!(writer, "{0:6}\t", state.line_number)?;
                state.line_number += 1;
            }

            // print to end of line or end of buffer
            let offset = if options.show_nonprint {
                write_nonprint_to_end(&in_buf[pos..], &mut writer, options.tab().as_bytes())
            } else if options.show_tabs {
                write_tab_to_end(&in_buf[pos..], &mut writer)
            } else {
                write_to_end(&in_buf[pos..], &mut writer)
            };
            // end of buffer?
            if offset + pos == in_buf.len() {
                state.at_line_start = false;
                break;
            }
            if in_buf[pos + offset] == b'\r' {
                state.skipped_carriage_return = true;
            } else {
                assert_eq!(in_buf[pos + offset], b'\n');
                // print suitable end of line
                writer.write_all(options.end_of_line().as_bytes())?;
                if handle.is_interactive {
                    writer.flush()?;
                }
                state.at_line_start = true;
            }
            pos += offset + 1;
        }
    }

    Ok(())
}

// write***_to_end methods
// Write all symbols till \n or \r or end of buffer is reached
// We need to stop at \r because it may be written as ^M depending on the byte after and settings;
// however, write_nonprint_to_end doesn't need to stop at \r because it will always write \r as ^M.
// Return the number of written symbols
fn write_to_end<W: Write>(in_buf: &[u8], writer: &mut W) -> usize {
    match in_buf.iter().position(|c| *c == b'\n' || *c == b'\r') {
        Some(p) => {
            writer.write_all(&in_buf[..p]).unwrap();
            p
        }
        None => {
            writer.write_all(in_buf).unwrap();
            in_buf.len()
        }
    }
}

fn write_tab_to_end<W: Write>(mut in_buf: &[u8], writer: &mut W) -> usize {
    let mut count = 0;
    loop {
        match in_buf
            .iter()
            .position(|c| *c == b'\n' || *c == b'\t' || *c == b'\r')
        {
            Some(p) => {
                writer.write_all(&in_buf[..p]).unwrap();
                if in_buf[p] == b'\t' {
                    writer.write_all(b"^I").unwrap();
                    in_buf = &in_buf[p + 1..];
                    count += p + 1;
                } else {
                    // b'\n' or b'\r'
                    return count + p;
                }
            }
            None => {
                writer.write_all(in_buf).unwrap();
                return in_buf.len();
            }
        };
    }
}

fn write_nonprint_to_end<W: Write>(in_buf: &[u8], writer: &mut W, tab: &[u8]) -> usize {
    let mut count = 0;

    for byte in in_buf.iter().copied() {
        if byte == b'\n' {
            break;
        }
        match byte {
            9 => writer.write_all(tab),
            0..=8 | 10..=31 => writer.write_all(&[b'^', byte + 64]),
            32..=126 => writer.write_all(&[byte]),
            127 => writer.write_all(&[b'^', b'?']),
            128..=159 => writer.write_all(&[b'M', b'-', b'^', byte - 64]),
            160..=254 => writer.write_all(&[b'M', b'-', byte - 128]),
            _ => writer.write_all(&[b'M', b'-', b'^', b'?']),
        }
        .unwrap();
        count += 1;
    }
    count
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
