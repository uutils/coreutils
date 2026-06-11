// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore nopipe

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Result, Write, stderr};
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::error::{UResult, strip_errno};
use uucore::translate;

mod cli;
pub use crate::cli::uu_app;
use crate::cli::{Options, OutputErrorMode, options};

#[cfg(target_os = "linux")]
use uucore::signals::ensure_stdout_not_broken;
#[cfg(unix)]
use uucore::signals::{disable_pipe_errors, ignore_interrupts};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let append = matches.get_flag(options::APPEND);
    let ignore_interrupts = matches.get_flag(options::IGNORE_INTERRUPTS);
    let ignore_pipe_errors = matches.get_flag(options::IGNORE_PIPE_ERRORS);
    let output_error = matches
        .get_one::<String>(options::OUTPUT_ERROR)
        .map(|s| match s.as_str() {
            "warn" => OutputErrorMode::Warn,
            "warn-nopipe" => OutputErrorMode::WarnNoPipe,
            "exit" => OutputErrorMode::Exit,
            "exit-nopipe" => OutputErrorMode::ExitNoPipe,
            _ => unreachable!("clap excluded it"),
        })
        .or_else(|| ignore_pipe_errors.then_some(OutputErrorMode::WarnNoPipe));

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

fn tee(options: &Options) -> Result<()> {
    #[cfg(unix)]
    {
        // ErrorKind::Other is raised by MultiWriter when all writers have exited.
        // This is therefore just a clever way to stop all writers

        if options.ignore_interrupts {
            ignore_interrupts().map_err(|_| Error::from(ErrorKind::Other))?;
        }
        if options.output_error.is_some() {
            disable_pipe_errors().map_err(|_| Error::from(ErrorKind::Other))?;
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
            #[cfg(any(unix, target_os = "wasi"))]
            inner: Writer::Stdout(uucore::io::RawWriter(rustix::stdio::stdout())),
            #[cfg(not(any(unix, target_os = "wasi")))]
            inner: Writer::Stdout(std::io::stdout()),
        },
    );

    let mut output = MultiWriter::new(writers, options.output_error);

    #[cfg(target_os = "linux")]
    if options.ignore_pipe_errors && !ensure_stdout_not_broken()? && output.writers.len() == 1 {
        return Ok(());
    }

    // don't use io::copy since content of 1 read should be immediately written for posix requirement
    let res = match output.copy_unbuffered() {
        // ErrorKind::Other is raised by MultiWriter when all writers
        // have exited, so that copy will abort. It's equivalent to
        // success of this part (if there was an error that should
        // cause a failure from any writer, that error would have been
        // returned instead).
        Err(e) if e.kind() != ErrorKind::Other => Err(e),
        _ => Ok(()),
    };

    if had_open_errors || res.is_err() || output.error_occurred() {
        Err(Error::from(ErrorKind::Other))
    } else {
        Ok(())
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
            inner: Writer::File(file),
            name: name.clone(),
        })),
        Err(f) => {
            let _ = writeln!(stderr(), "{}: {f}", name.maybe_quote());
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
    aborted: Option<Error>,
}

impl MultiWriter {
    /// Copies all bytes from the input buffer to the output buffer
    /// without buffering which is POSIX requirement.
    pub fn copy_unbuffered(&mut self) -> Result<()> {
        // todo: support splice() and tee() fast-path at here
        #[cfg(not(any(unix, target_os = "wasi")))]
        use std::io::Read as _;
        const BUF_SIZE: usize = 32 * 1024;
        #[cfg(any(unix, target_os = "wasi"))]
        let mut buf = [std::mem::MaybeUninit::<u8>::uninit(); BUF_SIZE];
        // todo: avoid cost by 0-fill keeping throughput
        #[cfg(not(any(unix, target_os = "wasi")))]
        let mut buf = [0u8; BUF_SIZE];

        let input = std::io::stdin();
        #[cfg(not(any(unix, target_os = "wasi")))]
        let mut input = input;
        loop {
            #[cfg(any(unix, target_os = "wasi"))]
            let res = rustix::io::read(&input, &mut buf)
                .map(|f| f.0)
                .map_err(Error::from);
            #[cfg(not(any(unix, target_os = "wasi")))]
            let res = input.read(&mut buf).map(|n| &buf[..n]);
            match res {
                Ok([]) => return Ok(()), // end of file
                Ok(slice) => self.write_flush(slice)?,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => {
                    let _ = writeln!(
                        stderr(),
                        "tee: {}",
                        translate!("tee-error-stdin", "error" => strip_errno(&e))
                    );
                    return Err(e);
                }
            }
        }
    }

    fn new(writers: Vec<NamedWriter>, output_error_mode: Option<OutputErrorMode>) -> Self {
        Self {
            writers,
            output_error_mode,
            ignored_errors: 0,
            aborted: None,
        }
    }

    fn error_occurred(&self) -> bool {
        self.ignored_errors != 0
    }

    fn write_flush(&mut self, buf: &[u8]) -> Result<()> {
        let mode = self.output_error_mode;
        self.writers
            .retain_mut(|writer| match writer.inner.write_all(buf) {
                Ok(()) => true,
                Err(e) => {
                    if let Err(e) = process_error(mode, e, writer, &mut self.ignored_errors) {
                        self.aborted.get_or_insert(e);
                    }
                    false
                }
            });
        match self.aborted.take() {
            Some(e) => Err(e),
            // This error kind will never be raised by std, so we can use it for termination when all writers exited
            None if self.writers.is_empty() => Err(Error::from(ErrorKind::Other)),
            None => Ok(()),
        }
    }
}

fn process_error(
    mode: Option<OutputErrorMode>,
    e: Error,
    writer: &NamedWriter,
    ignored_errors: &mut usize,
) -> Result<()> {
    let ignore_pipe = matches!(
        mode,
        None | Some(OutputErrorMode::WarnNoPipe) | Some(OutputErrorMode::ExitNoPipe)
    );

    if ignore_pipe && e.kind() == ErrorKind::BrokenPipe {
        return Ok(());
    }
    let _ = writeln!(stderr(), "{}: {e}", writer.name.maybe_quote());
    if let Some(OutputErrorMode::Exit | OutputErrorMode::ExitNoPipe) = mode {
        Err(e)
    } else {
        *ignored_errors += 1;
        Ok(())
    }
}

enum Writer {
    File(std::fs::File),
    // remove buffering for posix requirement and improve throughput
    #[cfg(any(unix, target_os = "wasi"))]
    Stdout(uucore::io::RawWriter<rustix::fd::BorrowedFd<'static>>),
    #[cfg(not(any(unix, target_os = "wasi")))]
    Stdout(std::io::Stdout),
}

impl Writer {
    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        match self {
            // File does not have line buffering
            Self::File(f) => f.write_all(buf),
            #[cfg(any(unix, target_os = "wasi"))]
            Self::Stdout(s) => s.write_all(buf),
            #[cfg(not(any(unix, target_os = "wasi")))]
            Self::Stdout(s) => {
                s.write_all(buf)?;
                // needs unsafe to remove buffering... flush after write_all to keep overhead minimal
                s.flush()
            }
        }
    }
}

struct NamedWriter {
    inner: Writer,
    pub name: OsString,
}
