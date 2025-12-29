// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore POLLERR POLLRDBAND pfds revents

use clap::{Arg, ArgAction, Command, builder::PossibleValue};
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write, stdin, stdout};
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::translate;
use uucore::{format_usage, show_error};

// spell-checker:ignore nopipe

#[cfg(unix)]
use uucore::signals::{enable_pipe_errors, ignore_interrupts};

mod options {
    pub const APPEND: &str = "append";
    pub const IGNORE_INTERRUPTS: &str = "ignore-interrupts";
    pub const FILE: &str = "file";
    pub const IGNORE_PIPE_ERRORS: &str = "ignore-pipe-errors";
    pub const OUTPUT_ERROR: &str = "output-error";
}

#[allow(dead_code)]
struct Options {
    append: bool,
    ignore_interrupts: bool,
    ignore_pipe_errors: bool,
    files: Vec<OsString>,
    output_error: Option<OutputErrorMode>,
}

#[derive(Clone, Debug)]
enum OutputErrorMode {
    /// Diagnose write error on any output
    Warn,
    /// Diagnose write error on any output that is not a pipe
    WarnNoPipe,
    /// Exit upon write error on any output
    Exit,
    /// Exit upon write error on any output that is not a pipe
    ExitNoPipe,
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let append = matches.get_flag(options::APPEND);
    let ignore_interrupts = matches.get_flag(options::IGNORE_INTERRUPTS);
    let ignore_pipe_errors = matches.get_flag(options::IGNORE_PIPE_ERRORS);
    let output_error = if matches.contains_id(options::OUTPUT_ERROR) {
        match matches
            .get_one::<String>(options::OUTPUT_ERROR)
            .map(String::as_str)
        {
            Some("warn") => Some(OutputErrorMode::Warn),
            // If no argument is specified for --output-error,
            // defaults to warn-nopipe
            None | Some("warn-nopipe") => Some(OutputErrorMode::WarnNoPipe),
            Some("exit") => Some(OutputErrorMode::Exit),
            Some("exit-nopipe") => Some(OutputErrorMode::ExitNoPipe),
            _ => unreachable!(),
        }
    } else if ignore_pipe_errors {
        Some(OutputErrorMode::WarnNoPipe)
    } else {
        None
    };

    let files = matches
        .get_many::<OsString>(options::FILE)
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    let options = Options {
        append,
        ignore_interrupts,
        ignore_pipe_errors,
        files,
        output_error,
    };

    tee(&options).map_err(|_| 1.into())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("tee-about"))
        .override_usage(format_usage(&translate!("tee-usage")))
        .after_help(translate!("tee-after-help"))
        .infer_long_args(true)
        // Since we use value-specific help texts for "--output-error", clap's "short help" and "long help" differ.
        // However, this is something that the GNU tests explicitly test for, so we *always* show the long help instead.
        .disable_help_flag(true)
        .arg(
            Arg::new("--help")
                .short('h')
                .long("help")
                .help(translate!("tee-help-help"))
                .action(ArgAction::HelpLong),
        )
        .arg(
            Arg::new(options::APPEND)
                .long(options::APPEND)
                .short('a')
                .help(translate!("tee-help-append"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short('i')
                .help(translate!("tee-help-ignore-interrupts"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::IGNORE_PIPE_ERRORS)
                .short('p')
                .help(translate!("tee-help-ignore-pipe-errors"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_ERROR)
                .long(options::OUTPUT_ERROR)
                .require_equals(true)
                .num_args(0..=1)
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("warn").help(translate!("tee-help-output-error-warn")),
                    PossibleValue::new("warn-nopipe")
                        .help(translate!("tee-help-output-error-warn-nopipe")),
                    PossibleValue::new("exit").help(translate!("tee-help-output-error-exit")),
                    PossibleValue::new("exit-nopipe")
                        .help(translate!("tee-help-output-error-exit-nopipe")),
                ]))
                .help(translate!("tee-help-output-error")),
        )
}

fn tee(options: &Options) -> Result<()> {
    #[cfg(unix)]
    {
        // ErrorKind::Other is raised by MultiWriter when all writers have exited.
        // This is therefore just a clever way to stop all writers

        if options.ignore_interrupts {
            ignore_interrupts().map_err(|_| Error::from(ErrorKind::Other))?;
        }
        if options.output_error.is_none() {
            enable_pipe_errors().map_err(|_| Error::from(ErrorKind::Other))?;
        }
    }
    let mut writers: Vec<NamedWriter> = options
        .files
        .iter()
        .filter_map(|file| open(file, options.append, options.output_error.as_ref()))
        .collect::<Result<Vec<NamedWriter>>>()?;
    let had_open_errors = writers.len() != options.files.len();

    writers.insert(
        0,
        NamedWriter {
            name: translate!("tee-standard-output").into(),
            inner: Box::new(stdout()),
        },
    );

    let mut output = MultiWriter::new(writers, options.output_error.clone());
    let input = &mut NamedReader {
        inner: Box::new(stdin()) as Box<dyn Read>,
    };

    #[cfg(target_os = "linux")]
    if options.ignore_pipe_errors && !ensure_stdout_not_broken()? && output.writers.len() == 1 {
        return Ok(());
    }

    // We cannot use std::io::copy here as it doesn't flush the output buffer
    let res = match copy(input, &mut output) {
        // ErrorKind::Other is raised by MultiWriter when all writers
        // have exited, so that copy will abort. It's equivalent to
        // success of this part (if there was an error that should
        // cause a failure from any writer, that error would have been
        // returned instead).
        Err(e) if e.kind() != ErrorKind::Other => Err(e),
        _ => Ok(()),
    };

    if had_open_errors || res.is_err() || output.flush().is_err() || output.error_occurred() {
        Err(Error::from(ErrorKind::Other))
    } else {
        Ok(())
    }
}

/// Copies all bytes from the input buffer to the output buffer.
///
/// Returns the number of written bytes.
fn copy(mut input: impl Read, mut output: impl Write) -> Result<usize> {
    // The implementation for this function is adopted from the generic buffer copy implementation from
    // the standard library:
    // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/io/copy.rs#L271-L297

    // Use buffer size from std implementation:
    // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/sys/io/mod.rs#L44
    // spell-checker:ignore espidf
    const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
        512
    } else {
        8 * 1024
    };

    let mut buffer = [0u8; DEFAULT_BUF_SIZE];
    let mut len = 0;

    loop {
        let received = match input.read(&mut buffer) {
            Ok(bytes_count) => bytes_count,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        if received == 0 {
            return Ok(len);
        }

        output.write_all(&buffer[0..received])?;

        // We need to flush the buffer here to comply with POSIX requirement that
        // `tee` does not buffer the input.
        output.flush()?;
        len += received;
    }
}

/// Tries to open the indicated file and return it. Reports an error if that's not possible.
/// If that error should lead to program termination, this function returns Some(Err()),
/// otherwise it returns None.
fn open(
    name: &OsString,
    append: bool,
    output_error: Option<&OutputErrorMode>,
) -> Option<Result<NamedWriter>> {
    let path = PathBuf::from(name);
    let mut options = OpenOptions::new();
    let mode = if append {
        options.append(true)
    } else {
        options.truncate(true)
    };
    match mode.write(true).create(true).open(path.as_path()) {
        Ok(file) => Some(Ok(NamedWriter {
            inner: Box::new(file),
            name: name.clone(),
        })),
        Err(f) => {
            show_error!("{}: {f}", name.maybe_quote());
            match output_error {
                Some(OutputErrorMode::Exit | OutputErrorMode::ExitNoPipe) => Some(Err(f)),
                _ => None,
            }
        }
    }
}

struct MultiWriter {
    writers: Vec<NamedWriter>,
    output_error_mode: Option<OutputErrorMode>,
    ignored_errors: usize,
}

impl MultiWriter {
    fn new(writers: Vec<NamedWriter>, output_error_mode: Option<OutputErrorMode>) -> Self {
        Self {
            writers,
            output_error_mode,
            ignored_errors: 0,
        }
    }

    fn error_occurred(&self) -> bool {
        self.ignored_errors != 0
    }
}

fn process_error(
    mode: Option<&OutputErrorMode>,
    f: Error,
    writer: &NamedWriter,
    ignored_errors: &mut usize,
) -> Result<()> {
    match mode {
        Some(OutputErrorMode::Warn) => {
            show_error!("{}: {f}", writer.name.maybe_quote());
            *ignored_errors += 1;
            Ok(())
        }
        Some(OutputErrorMode::WarnNoPipe) | None => {
            if f.kind() != ErrorKind::BrokenPipe {
                show_error!("{}: {f}", writer.name.maybe_quote());
                *ignored_errors += 1;
            }
            Ok(())
        }
        Some(OutputErrorMode::Exit) => {
            show_error!("{}: {f}", writer.name.maybe_quote());
            Err(f)
        }
        Some(OutputErrorMode::ExitNoPipe) => {
            if f.kind() == ErrorKind::BrokenPipe {
                Ok(())
            } else {
                show_error!("{}: {f}", writer.name.maybe_quote());
                Err(f)
            }
        }
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut aborted = None;
        let mode = self.output_error_mode.clone();
        let mut errors = 0;
        self.writers.retain_mut(|writer| {
            let result = writer.write_all(buf);
            match result {
                Err(f) => {
                    if let Err(e) = process_error(mode.as_ref(), f, writer, &mut errors) {
                        if aborted.is_none() {
                            aborted = Some(e);
                        }
                    }
                    false
                }
                _ => true,
            }
        });
        self.ignored_errors += errors;
        if let Some(e) = aborted {
            Err(e)
        } else if self.writers.is_empty() {
            // This error kind will never be raised by the standard
            // library, so we can use it for early termination of
            // `copy`
            Err(Error::from(ErrorKind::Other))
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
        let mut aborted = None;
        let mode = self.output_error_mode.clone();
        let mut errors = 0;
        self.writers.retain_mut(|writer| {
            let result = writer.flush();
            match result {
                Err(f) => {
                    if let Err(e) = process_error(mode.as_ref(), f, writer, &mut errors) {
                        if aborted.is_none() {
                            aborted = Some(e);
                        }
                    }
                    false
                }
                _ => true,
            }
        });
        self.ignored_errors += errors;
        if let Some(e) = aborted {
            Err(e)
        } else {
            Ok(())
        }
    }
}

struct NamedWriter {
    inner: Box<dyn Write>,
    pub name: OsString,
}

impl Write for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

struct NamedReader {
    inner: Box<dyn Read>,
}

impl Read for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.inner.read(buf) {
            Err(f) => {
                show_error!("{}", translate!("tee-error-stdin", "error" => f));
                Err(f)
            }
            okay => okay,
        }
    }
}

/// Check that if stdout is a pipe, it is not broken.
#[cfg(target_os = "linux")]
pub fn ensure_stdout_not_broken() -> Result<bool> {
    use nix::{
        poll::{PollFd, PollFlags, PollTimeout},
        sys::stat::{SFlag, fstat},
    };
    use std::os::fd::AsFd;

    let out = stdout();

    // First, check that stdout is a fifo and return true if it's not the case
    let stat = fstat(out.as_fd())?;
    if !SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFIFO) {
        return Ok(true);
    }

    // POLLRDBAND is the flag used by GNU tee.
    let mut pfds = [PollFd::new(out.as_fd(), PollFlags::POLLRDBAND)];

    // Then, ensure that the pipe is not broken.
    // Use ZERO timeout to return immediately - we just want to check the current state.
    let res = nix::poll::poll(&mut pfds, PollTimeout::ZERO)?;

    if res > 0 {
        // poll returned with events ready - check if POLLERR is set (pipe broken)
        let error = pfds.iter().any(|pfd| {
            if let Some(revents) = pfd.revents() {
                revents.contains(PollFlags::POLLERR)
            } else {
                true
            }
        });
        return Ok(!error);
    }

    // res == 0 means no events ready (timeout reached immediately with ZERO timeout).
    // This means the pipe is healthy (not broken).
    // res < 0 would be an error, but nix returns Err in that case.
    Ok(true)
}
