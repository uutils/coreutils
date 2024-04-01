// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::PossibleValue, crate_version, Arg, ArgAction, Command};
use std::fs::OpenOptions;
use std::io::{copy, stdin, stdout, Error, ErrorKind, Read, Result, Write};
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage, show_error};

// spell-checker:ignore nopipe

#[cfg(unix)]
use uucore::signals::{enable_pipe_errors, ignore_interrupts};

const ABOUT: &str = help_about!("tee.md");
const USAGE: &str = help_usage!("tee.md");
const AFTER_HELP: &str = help_section!("after help", "tee.md");

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
    files: Vec<String>,
    output_error: Option<OutputErrorMode>,
}

#[derive(Clone, Debug)]
enum OutputErrorMode {
    Warn,
    WarnNoPipe,
    Exit,
    ExitNoPipe,
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let options = Options {
        append: matches.get_flag(options::APPEND),
        ignore_interrupts: matches.get_flag(options::IGNORE_INTERRUPTS),
        files: matches
            .get_many::<String>(options::FILE)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default(),
        output_error: {
            if matches.get_flag(options::IGNORE_PIPE_ERRORS) {
                Some(OutputErrorMode::WarnNoPipe)
            } else if matches.contains_id(options::OUTPUT_ERROR) {
                if let Some(v) = matches.get_one::<String>(options::OUTPUT_ERROR) {
                    match v.as_str() {
                        "warn" => Some(OutputErrorMode::Warn),
                        "warn-nopipe" => Some(OutputErrorMode::WarnNoPipe),
                        "exit" => Some(OutputErrorMode::Exit),
                        "exit-nopipe" => Some(OutputErrorMode::ExitNoPipe),
                        _ => unreachable!(),
                    }
                } else {
                    Some(OutputErrorMode::WarnNoPipe)
                }
            } else {
                None
            }
        },
    };

    match tee(&options) {
        Ok(_) => Ok(()),
        Err(_) => Err(1.into()),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .arg(
            Arg::new(options::APPEND)
                .long(options::APPEND)
                .short('a')
                .help("append to the given FILEs, do not overwrite")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short('i')
                .help("ignore interrupt signals (ignored on non-Unix platforms)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::IGNORE_PIPE_ERRORS)
                .short('p')
                .help("set write error behavior (ignored on non-Unix platforms)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_ERROR)
                .long(options::OUTPUT_ERROR)
                .require_equals(true)
                .num_args(0..=1)
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("warn")
                        .help("produce warnings for errors writing to any output"),
                    PossibleValue::new("warn-nopipe")
                        .help("produce warnings for errors that are not pipe errors (ignored on non-unix platforms)"),
                    PossibleValue::new("exit").help("exit on write errors to any output"),
                    PossibleValue::new("exit-nopipe")
                        .help("exit on write errors to any output that are not pipe errors (equivalent to exit on non-unix platforms)"),
                ]))
                .help("set write error behavior")
                .conflicts_with(options::IGNORE_PIPE_ERRORS),
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
            name: "'standard output'".to_owned(),
            inner: Box::new(stdout()),
        },
    );

    let mut output = MultiWriter::new(writers, options.output_error.clone());
    let input = &mut NamedReader {
        inner: Box::new(stdin()) as Box<dyn Read>,
    };

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

/// Tries to open the indicated file and return it. Reports an error if that's not possible.
/// If that error should lead to program termination, this function returns Some(Err()),
/// otherwise it returns None.
fn open(
    name: &str,
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
            name: name.to_owned(),
        })),
        Err(f) => {
            show_error!("{}: {}", name.maybe_quote(), f);
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
            show_error!("{}: {}", writer.name.maybe_quote(), f);
            *ignored_errors += 1;
            Ok(())
        }
        Some(OutputErrorMode::WarnNoPipe) | None => {
            if f.kind() != ErrorKind::BrokenPipe {
                show_error!("{}: {}", writer.name.maybe_quote(), f);
                *ignored_errors += 1;
            }
            Ok(())
        }
        Some(OutputErrorMode::Exit) => {
            show_error!("{}: {}", writer.name.maybe_quote(), f);
            Err(f)
        }
        Some(OutputErrorMode::ExitNoPipe) => {
            if f.kind() == ErrorKind::BrokenPipe {
                Ok(())
            } else {
                show_error!("{}: {}", writer.name.maybe_quote(), f);
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
    pub name: String,
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
                show_error!("stdin: {}", f);
                Err(f)
            }
            okay => okay,
        }
    }
}
