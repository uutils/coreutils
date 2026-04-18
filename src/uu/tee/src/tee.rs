// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore espidf nopipe

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write, stderr, stdin, stdout};
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
            inner: Writer::Stdout(stdout()),
        },
    );

    let mut output = MultiWriter::new(writers, options.output_error.clone());
    let input = NamedReader { inner: stdin() };

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
fn copy(mut input: impl Read, mut output: impl Write) -> Result<()> {
    // The implementation for this function is adopted from the generic buffer copy implementation from
    // the standard library:
    // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/io/copy.rs#L271-L297

    // Use buffer size from std implementation
    // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/sys/io/mod.rs#L44
    const BUF_SIZE: usize = 8 * 1024;
    let mut buffer = [0u8; BUF_SIZE];

    for _ in 0..2 {
        match input.read(&mut buffer) {
            Ok(0) => return Ok(()), // end of file
            Ok(received) => {
                output.write_all(&buffer[..received])?;
                // flush the buffer to comply with POSIX requirement that
                // `tee` does not buffer the input.
                output.flush()?;
            }
            Err(e) if e.kind() != ErrorKind::Interrupted => return Err(e),
            _ => {}
        }
    }
    // buffer is too small optimize for large input
    //stack array makes code path for smaller file slower
    let mut buffer = vec![0u8; 4 * BUF_SIZE];
    loop {
        match input.read(&mut buffer) {
            Ok(0) => return Ok(()), // end of file
            Ok(received) => {
                output.write_all(&buffer[..received])?;
                // flush the buffer to comply with POSIX requirement that
                // `tee` does not buffer the input.
                output.flush()?;
            }
            Err(e) if e.kind() != ErrorKind::Interrupted => return Err(e),
            _ => {}
        }
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
    let ignore_pipe = matches!(
        mode,
        None | Some(OutputErrorMode::WarnNoPipe) | Some(OutputErrorMode::ExitNoPipe)
    );

    if ignore_pipe && f.kind() == ErrorKind::BrokenPipe {
        return Ok(());
    }
    let _ = writeln!(stderr(), "{}: {f}", writer.name.maybe_quote());
    if let Some(OutputErrorMode::Exit | OutputErrorMode::ExitNoPipe) = mode {
        Err(f)
    } else {
        *ignored_errors += 1;
        Ok(())
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut aborted = None;
        let mode = self.output_error_mode.clone();
        let mut errors = 0;
        self.writers.retain_mut(|writer| {
            writer
                .write_all(buf)
                .map_err(|f| {
                    let _ = process_error(mode.as_ref(), f, writer, &mut errors)
                        .map_err(|e| aborted.get_or_insert(e));
                })
                .is_ok()
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
            writer
                .flush()
                .map_err(|f| {
                    let _ = process_error(mode.as_ref(), f, writer, &mut errors)
                        .map_err(|e| aborted.get_or_insert(e));
                })
                .is_ok()
        });
        self.ignored_errors += errors;
        aborted.map_or(Ok(()), Err)
    }
}

enum Writer {
    File(std::fs::File),
    Stdout(std::io::Stdout),
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::File(f) => f.write(buf),
            Self::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::File(f) => f.flush(),
            Self::Stdout(s) => s.flush(),
        }
    }
}

struct NamedWriter {
    inner: Writer,
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
    inner: std::io::Stdin,
}

impl Read for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf).inspect_err(|e| {
            let _ = writeln!(
                stderr(),
                "tee: {}",
                translate!("tee-error-stdin", "error" => strip_errno(e))
            );
        })
    }
}
