// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) nonprint nonblank nonprinting ELOOP

mod platform;

use crate::platform::is_unsafe_overwrite;
use clap::{Arg, ArgAction, Command};
use memchr::memchr2;
use std::ffi::OsString;
use std::fs::{File, metadata};
use std::io::{self, BufWriter, ErrorKind, IsTerminal, Read, Write};
/// Unix domain socket support
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::fd::AsFd;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::UResult;
#[cfg(not(target_os = "windows"))]
use uucore::libc;
use uucore::translate;
use uucore::{fast_inc::fast_inc_one, format_usage};

/// Linux splice support
#[cfg(any(target_os = "linux", target_os = "android"))]
mod splice;

// Allocate 32 digits for the line number.
// An estimate is that we can print about 1e8 lines/seconds, so 32 digits
// would be enough for billions of universe lifetimes.
const LINE_NUMBER_BUF_SIZE: usize = 32;

struct LineNumber {
    buf: [u8; LINE_NUMBER_BUF_SIZE],
    print_start: usize,
    num_start: usize,
    num_end: usize,
}

// Logic to store a string for the line number. Manually incrementing the value
// represented in a buffer like this is significantly faster than storing
// a `usize` and using the standard Rust formatting macros to format a `usize`
// to a string each time it's needed.
// Buffer is initialized to "     1\t" and incremented each time `increment` is
// called, using uucore's fast_inc function that operates on strings.
impl LineNumber {
    fn new() -> Self {
        let mut buf = [b'0'; LINE_NUMBER_BUF_SIZE];

        let init_str = "     1\t";
        let print_start = buf.len() - init_str.len();
        let num_start = buf.len() - 2;
        let num_end = buf.len() - 1;

        buf[print_start..].copy_from_slice(init_str.as_bytes());

        Self {
            buf,
            print_start,
            num_start,
            num_end,
        }
    }

    fn increment(&mut self) {
        fast_inc_one(&mut self.buf, &mut self.num_start, self.num_end);
        self.print_start = self.print_start.min(self.num_start);
    }

    #[inline]
    fn to_str(&self) -> &[u8] {
        &self.buf[self.print_start..]
    }

    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(self.to_str())
    }
}

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
    #[error("{}", translate!("cat-error-unknown-filetype", "ft_debug" => .ft_debug))]
    UnknownFiletype {
        /// A debug print of the file type
        ft_debug: String,
    },
    #[error("{}", translate!("cat-error-is-directory"))]
    IsDirectory,
    #[error("{}", translate!("cat-error-input-file-is-output-file"))]
    OutputIsInput,
    #[error("{}", translate!("cat-error-too-many-symbolic-links"))]
    TooManySymlinks,
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
        if self.show_tabs { "^I" } else { "\t" }
    }

    fn end_of_line(&self) -> &'static str {
        if self.show_ends { "$\n" } else { "\n" }
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
    line_number: LineNumber,

    /// Whether the output cursor is at the beginning of a new line
    at_line_start: bool,

    /// Whether we skipped a \r, which still needs to be printed
    skipped_carriage_return: bool,

    /// Whether we have already printed a blank line
    one_blank_kept: bool,
}

#[cfg(unix)]
trait FdReadable: Read + AsFd {}
#[cfg(not(unix))]
trait FdReadable: Read {}

#[cfg(unix)]
impl<T> FdReadable for T where T: Read + AsFd {}
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
/// [`CatError::IsDirectory`]
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
    pub static IGNORED_U: &str = "ignored-u";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // When we receive a SIGPIPE signal, we want to terminate the process so
    // that we don't print any error messages to stderr. Rust ignores SIGPIPE
    // (see https://github.com/rust-lang/rust/issues/62569), so we restore it's
    // default action here.
    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let number_mode = if matches.get_flag(options::NUMBER_NONBLANK) {
        NumberingMode::NonEmpty
    } else if matches.get_flag(options::NUMBER) {
        NumberingMode::All
    } else {
        NumberingMode::None
    };

    let show_nonprint = [
        options::SHOW_ALL.to_owned(),
        options::SHOW_NONPRINTING_ENDS.to_owned(),
        options::SHOW_NONPRINTING_TABS.to_owned(),
        options::SHOW_NONPRINTING.to_owned(),
    ]
    .iter()
    .any(|v| matches.get_flag(v));

    let show_ends = [
        options::SHOW_ENDS.to_owned(),
        options::SHOW_ALL.to_owned(),
        options::SHOW_NONPRINTING_ENDS.to_owned(),
    ]
    .iter()
    .any(|v| matches.get_flag(v));

    let show_tabs = [
        options::SHOW_ALL.to_owned(),
        options::SHOW_TABS.to_owned(),
        options::SHOW_NONPRINTING_TABS.to_owned(),
    ]
    .iter()
    .any(|v| matches.get_flag(v));

    let squeeze_blank = matches.get_flag(options::SQUEEZE_BLANK);
    let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec![OsString::from("-")],
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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(format_usage(&translate!("cat-usage")))
        .about(translate!("cat-about"))
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SHOW_ALL)
                .short('A')
                .long(options::SHOW_ALL)
                .help(translate!("cat-help-show-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER_NONBLANK)
                .short('b')
                .long(options::NUMBER_NONBLANK)
                .help(translate!("cat-help-number-nonblank"))
                // Note: This MUST NOT .overrides_with(options::NUMBER)!
                // In clap, overriding is symmetric, so "-b -n" counts as "-n", which is not what we want.
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_ENDS)
                .short('e')
                .help(translate!("cat-help-show-nonprinting-ends"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_ENDS)
                .short('E')
                .long(options::SHOW_ENDS)
                .help(translate!("cat-help-show-ends"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER)
                .short('n')
                .long(options::NUMBER)
                .help(translate!("cat-help-number"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE_BLANK)
                .short('s')
                .long(options::SQUEEZE_BLANK)
                .help(translate!("cat-help-squeeze-blank"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_TABS)
                .short('t')
                .help(translate!("cat-help-show-nonprinting-tabs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_TABS)
                .short('T')
                .long(options::SHOW_TABS)
                .help(translate!("cat-help-show-tabs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING)
                .short('v')
                .long(options::SHOW_NONPRINTING)
                .help(translate!("cat-help-show-nonprinting"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORED_U)
                .short('u')
                .help(translate!("cat-help-ignored-u"))
                .action(ArgAction::SetTrue),
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

fn cat_path(path: &OsString, options: &OutputOptions, state: &mut OutputState) -> CatResult<()> {
    match get_input_type(path)? {
        InputType::StdIn => {
            let stdin = io::stdin();
            if is_unsafe_overwrite(&stdin, &io::stdout()) {
                return Err(CatError::OutputIsInput);
            }
            let mut handle = InputHandle {
                reader: stdin,
                is_interactive: io::stdin().is_terminal(),
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
            if is_unsafe_overwrite(&file, &io::stdout()) {
                return Err(CatError::OutputIsInput);
            }
            let mut handle = InputHandle {
                reader: file,
                is_interactive: false,
            };
            cat_handle(&mut handle, options, state)
        }
    }
}

fn cat_files(files: &[OsString], options: &OutputOptions) -> UResult<()> {
    let mut state = OutputState {
        line_number: LineNumber::new(),
        at_line_start: true,
        skipped_carriage_return: false,
        one_blank_kept: false,
    };
    let mut error_messages: Vec<String> = Vec::new();

    for path in files {
        if let Err(err) = cat_path(path, options, &mut state) {
            error_messages.push(format!("{}: {err}", path.maybe_quote()));
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
fn get_input_type(path: &OsString) -> CatResult<InputType> {
    if path == "-" {
        return Ok(InputType::StdIn);
    }

    let ft = match metadata(path) {
        Ok(md) => md.file_type(),
        Err(e) => {
            if let Some(raw_error) = e.raw_os_error() {
                // On Unix-like systems, the error code for "Too many levels of symbolic links" is 40 (ELOOP).
                // we want to provide a proper error message in this case.
                #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
                let too_many_symlink_code = 40;
                #[cfg(any(target_os = "macos", target_os = "freebsd"))]
                let too_many_symlink_code = 62;
                if raw_error == too_many_symlink_code {
                    return Err(CatError::TooManySymlinks);
                }
            }
            return Err(CatError::Io(e));
        }
    };
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
            ft_debug: format!("{ft:?}"),
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
    loop {
        match handle.reader.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                stdout_lock
                    .write_all(&buf[..n])
                    .inspect_err(handle_broken_pipe)?;
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }

    // If the splice() call failed and there has been some data written to
    // stdout via while loop above AND there will be second splice() call
    // that will succeed, data pushed through splice will be output before
    // the data buffered in stdout.lock. Therefore additional explicit flush
    // is required here.
    stdout_lock.flush().inspect_err(handle_broken_pipe)?;
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
    let stdout = stdout.lock();
    // Add a 32K buffer for stdout - this greatly improves performance.
    let mut writer = BufWriter::with_capacity(32 * 1024, stdout);

    loop {
        let n = match handle.reader.read(&mut in_buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        let in_buf = &in_buf[..n];
        let mut pos = 0;
        while pos < n {
            // skip empty line_number enumerating them if needed
            if in_buf[pos] == b'\n' {
                write_new_line(&mut writer, options, state, handle.is_interactive)?;
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
                state.line_number.write(&mut writer)?;
                state.line_number.increment();
            }

            // print to end of line or end of buffer
            let offset = write_end(&mut writer, &in_buf[pos..], options);

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
                write_end_of_line(
                    &mut writer,
                    options.end_of_line().as_bytes(),
                    handle.is_interactive,
                )?;
                state.at_line_start = true;
            }
            pos += offset + 1;
        }
        // We need to flush the buffer each time around the loop in order to pass GNU tests.
        // When we are reading the input from a pipe, the `handle.reader.read` call at the top
        // of this loop will block (indefinitely) whist waiting for more data. The expectation
        // however is that anything that's ready for output should show up in the meantime,
        // and not be buffered internally to the `cat` process.
        // Hence it's necessary to flush our buffer before every time we could potentially block
        // on a `std::io::Read::read` call.
        writer.flush().inspect_err(handle_broken_pipe)?;
    }

    Ok(())
}

/// `\r` followed by `\n` is printed as `^M` when `show_ends` is enabled, so that `\r\n` prints as `^M$`
fn write_new_line<W: Write>(
    writer: &mut W,
    options: &OutputOptions,
    state: &mut OutputState,
    is_interactive: bool,
) -> CatResult<()> {
    if state.skipped_carriage_return {
        if options.show_ends {
            writer.write_all(b"^M")?;
        } else {
            writer.write_all(b"\r")?;
        }
        state.skipped_carriage_return = false;

        write_end_of_line(writer, options.end_of_line().as_bytes(), is_interactive)?;
        return Ok(());
    }
    if !state.at_line_start || !options.squeeze_blank || !state.one_blank_kept {
        state.one_blank_kept = true;
        if state.at_line_start && options.number == NumberingMode::All {
            state.line_number.write(writer)?;
            state.line_number.increment();
        }
        write_end_of_line(writer, options.end_of_line().as_bytes(), is_interactive)?;
    }
    Ok(())
}

fn write_end<W: Write>(writer: &mut W, in_buf: &[u8], options: &OutputOptions) -> usize {
    if options.show_nonprint {
        write_nonprint_to_end(in_buf, writer, options.tab().as_bytes())
    } else if options.show_tabs {
        write_tab_to_end(in_buf, writer)
    } else {
        write_to_end(in_buf, writer)
    }
}

// write***_to_end methods
// Write all symbols till \n or \r or end of buffer is reached
// We need to stop at \r because it may be written as ^M depending on the byte after and settings;
// however, write_nonprint_to_end doesn't need to stop at \r because it will always write \r as ^M.
// Return the number of written symbols

fn write_to_end<W: Write>(in_buf: &[u8], writer: &mut W) -> usize {
    // using memchr2 significantly improves performances
    match memchr2(b'\n', b'\r', in_buf) {
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
                return in_buf.len() + count;
            }
        }
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
            127 => writer.write_all(b"^?"),
            128..=159 => writer.write_all(&[b'M', b'-', b'^', byte - 64]),
            160..=254 => writer.write_all(&[b'M', b'-', byte - 128]),
            _ => writer.write_all(b"M-^?"),
        }
        .unwrap();
        count += 1;
    }
    count
}

fn write_end_of_line<W: Write>(
    writer: &mut W,
    end_of_line: &[u8],
    is_interactive: bool,
) -> CatResult<()> {
    writer.write_all(end_of_line)?;
    if is_interactive {
        writer.flush().inspect_err(handle_broken_pipe)?;
    }
    Ok(())
}

fn handle_broken_pipe(error: &io::Error) {
    // SIGPIPE is not available on Windows.
    if cfg!(target_os = "windows") && error.kind() == ErrorKind::BrokenPipe {
        std::process::exit(13);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufWriter, stdout};

    #[test]
    fn test_write_tab_to_end_with_newline() {
        let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
        let in_buf = b"a\tb\tc\n";
        assert_eq!(super::write_tab_to_end(in_buf, &mut writer), 5);
    }

    #[test]
    fn test_write_tab_to_end_no_newline() {
        let mut writer = BufWriter::with_capacity(1024 * 64, stdout());
        let in_buf = b"a\tb\tc";
        assert_eq!(super::write_tab_to_end(in_buf, &mut writer), 5);
    }

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

    #[test]
    fn test_incrementing_string() {
        let mut incrementing_string = super::LineNumber::new();
        assert_eq!(b"     1\t", incrementing_string.to_str());
        incrementing_string.increment();
        assert_eq!(b"     2\t", incrementing_string.to_str());
        // Run through to 100
        for _ in 3..=100 {
            incrementing_string.increment();
        }
        assert_eq!(b"   100\t", incrementing_string.to_str());
        // Run through until we overflow the original size.
        for _ in 101..=1_000_000 {
            incrementing_string.increment();
        }
        // Confirm that the start position moves when we overflow the original size.
        assert_eq!(b"1000000\t", incrementing_string.to_str());
        incrementing_string.increment();
        assert_eq!(b"1000001\t", incrementing_string.to_str());
    }
}
