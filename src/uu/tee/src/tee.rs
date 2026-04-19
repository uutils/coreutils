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

    let mut output = MultiWriter::new(writers, options.output_error);
    let input = NamedReader { inner: stdin() };

    #[cfg(target_os = "linux")]
    if options.ignore_pipe_errors && !ensure_stdout_not_broken()? && output.writers.len() == 1 {
        return Ok(());
    }

    // We cannot use std::io::copy here as it doesn't flush the output buffer
    let res = match output.copy_unbuffered(input) {
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
}

impl MultiWriter {
    /// Copies all bytes from the input buffer to the output buffer
    /// without buffering which is POSIX requirement.
    pub fn copy_unbuffered(&mut self, mut input: NamedReader) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            use ::rustix::fd::AsFd;
            use uucore::pipes::{MAX_ROOTLESS_PIPE_SIZE, pipe, splice, splice_exact};
            let (pipe_read, pipe_write) = pipe()?; // needed to duplicate input
            let (pipe2_read, pipe2_write) = pipe()?; // force-tee() even output is not pipe
            let input = input.inner.as_fd();
            let mode = self.output_error_mode;
            // improve throughput
            let _ = rustix::pipe::fcntl_setpipe_size(
                self.writers[0].inner.as_fd(),
                MAX_ROOTLESS_PIPE_SIZE,
            );
            'splice: loop {
                let mut aborted = None;
                match splice(&input, &pipe_write, MAX_ROOTLESS_PIPE_SIZE) {
                    Ok(0) => return Ok(()),
                    Err(_) => break 'splice,
                    Ok(s) => {
                        let w_len = self.writers.len();
                        // len - 1 outputs do not consume input
                        for other in &mut self.writers[..w_len - 1] {
                            assert_eq!(
                                uucore::pipes::tee(&pipe_read, &pipe2_write, s),
                                Ok(s),
                                "tee() between internal pipes should not be blocked"
                            );
                            let fd = other.inner.as_fd();
                            if splice_exact(&pipe2_read, &fd, s).is_err() {
                                // fallback with proper error message
                                debug_assert!(s <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                                let mut drain = Vec::with_capacity(s);
                                let mut reader = (&pipe2_read).take(s as u64);
                                let res = (|| {
                                    reader.read_to_end(&mut drain)?;
                                    other.inner.write_all(&drain)?;
                                    other.inner.flush()
                                })();
                                if let Err(e) = res {
                                    if let Err(e) =
                                        process_error(mode, e, other, &mut self.ignored_errors)
                                    {
                                        aborted.get_or_insert(e);
                                    }
                                    other.name.clear(); //mark as exited
                                }
                            }
                        }
                        // last one consumes input
                        if let Some(last) = self.writers.last_mut() {
                            if splice_exact(&pipe_read, &last.inner.as_fd(), s).is_err() {
                                // fallback with proper error message
                                debug_assert!(s <= MAX_ROOTLESS_PIPE_SIZE, "unexpected RAM usage");
                                let mut drain = Vec::with_capacity(s);
                                let mut reader = (&pipe_read).take(s as u64);
                                let res = (|| {
                                    reader.read_to_end(&mut drain)?;
                                    last.inner.write_all(&drain)?;
                                    last.inner.flush()
                                })();
                                if let Err(e) = res {
                                    if let Err(e) =
                                        process_error(mode, e, last, &mut self.ignored_errors)
                                    {
                                        aborted.get_or_insert(e);
                                    }
                                    last.name.clear(); //mark as exited
                                }
                            }
                        }
                    }
                }
                self.writers.retain(|w| !w.name.is_empty());
                if let Some(e) = aborted {
                    return Err(e);
                }
                if self.writers.is_empty() {
                    return Err(Error::from(ErrorKind::Other));
                }
            }
        }
        // The implementation for this function is adopted from the generic buffer copy implementation from
        // the standard library:
        // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/io/copy.rs#L271-L297

        // Use buffer size from std implementation
        // https://github.com/rust-lang/rust/blob/2feb91181882e525e698c4543063f4d0296fcf91/library/std/src/sys/io/mod.rs#L44
        const BUF_SIZE: usize = 8 * 1024;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        let mut buffer = [0u8; BUF_SIZE];
        // fast-path for small input on the platform missing splice
        // needs 2+ read to catch end of file
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        for _ in 0..2 {
            match input.read(&mut buffer) {
                Ok(0) => return Ok(()), // end of file
                Ok(received) => self.write_flush(&buffer[..received])?,
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
                Ok(received) => self.write_flush(&buffer[..received])?,
                Err(e) if e.kind() != ErrorKind::Interrupted => return Err(e),
                _ => {}
            }
        }
    }

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

    fn write_flush(&mut self, buf: &[u8]) -> Result<()> {
        let mut aborted = None;
        let mode = self.output_error_mode;
        self.writers.retain_mut(|writer| {
            let res = (|| {
                writer.inner.write_all(buf)?;
                writer.inner.flush()
            })();
            match res {
                Ok(()) => true,
                Err(e) => {
                    if let Err(e) = process_error(mode, e, writer, &mut self.ignored_errors) {
                        aborted.get_or_insert(e);
                    }
                    false
                }
            }
        });
        aborted.map_or(
            if self.writers.is_empty() {
                // This error kind will never be raised by the standard
                // library, so we can use it for early termination of
                // `copy`
                Err(Error::from(ErrorKind::Other))
            } else {
                Ok(())
            },
            Err,
        )
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

#[cfg(any(target_os = "linux", target_os = "android"))]
impl rustix::fd::AsFd for Writer {
    fn as_fd(&self) -> rustix::fd::BorrowedFd<'_> {
        match self {
            Self::File(f) => f.as_fd(),
            Self::Stdout(s) => s.as_fd(),
        }
    }
}
