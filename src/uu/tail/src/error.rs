// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore Uncategorized stdlib untailable tailable

use crate::paths::Input;
#[cfg(unix)]
use crate::text;
use crate::{follow::Observer, paths::MetadataExtTail};
use std::{fs::File, io};
use uucore::error::{set_exit_code, FromIo, UIoError, UResult, USimpleError};

#[cfg(windows)]
pub fn new_io_directory_error() -> io::Error {
    io::Error::from_raw_os_error(336)
}

#[cfg(not(windows))]
pub fn new_io_directory_error() -> io::Error {
    io::Error::from_raw_os_error(21)
}

#[derive(Debug)]
pub enum TailError {
    Stat(io::Error),
    Open(io::Error),
    Read(io::Error),
    Write(io::Error),
}

/// Print and format the error message of an [`TailError`].
///
/// Can be used for testing purposes, to store the error message instead of printing to `stderr`.
pub struct TailErrorPrinter {
    display_name: String,
    prefix: String,
}

impl TailErrorPrinter {
    pub fn new(display_name: &str) -> Self {
        Self {
            display_name: String::from(display_name),
            prefix: String::from(uucore::util_name()),
        }
    }

    pub fn format<U: AsRef<str>, T: AsRef<str>>(
        &self,
        reason: Option<U>,
        message: T,
        raw: bool,
    ) -> String {
        if raw {
            format!("{}: {}", self.prefix, message.as_ref())
        } else {
            let reason = reason.map_or(
                format!("cannot open '{}' for reading", self.display_name),
                |r| format!("{} '{}'", r.as_ref(), self.display_name),
            );
            format!("{}: {}: {}", self.prefix, reason, message.as_ref())
        }
    }

    pub fn print<U: AsRef<str>, T: AsRef<str>>(&self, reason: &Option<U>, message: T, raw: bool) {
        let string = self.format(reason.as_ref(), message.as_ref(), raw);
        eprintln!("{}", string);
    }

    pub fn print_message<T: AsRef<str>>(&self, message: T) {
        self.print::<String, T>(&None, message, true);
    }

    pub fn print_error(&self, error: &TailError) {
        let message = error
            .map_err_context(|| match error {
                TailError::Stat(_) => format!("cannot fstat '{}'", self.display_name),
                TailError::Open(_) => format!("cannot open '{}' for reading", self.display_name),
                TailError::Read(_) => format!("error reading '{}'", self.display_name),
                TailError::Write(_) => "write error".to_string(),
            })
            .to_string();

        self.print_message(message);
    }
}

impl FromIo<Box<UIoError>> for &TailError {
    fn map_err_context(self, context: impl FnOnce() -> String) -> Box<UIoError> {
        let io_error = match self {
            TailError::Stat(error)
            | TailError::Open(error)
            | TailError::Read(error)
            | TailError::Write(error) => {
                if let Some(raw) = error.raw_os_error() {
                    io::Error::from_raw_os_error(raw)
                } else {
                    error.kind().into()
                }
            }
        };
        io_error.map_err_context(context)
    }
}

/// Handle a [`TailError`].
///
/// This includes setting the exit code and printing of the error message. This handler needs an
/// [`Input`] to print the correct error message.
pub struct TailErrorHandler {
    input: Input,
    printer: TailErrorPrinter,
    follow: bool,
    retry: bool,
}

impl TailErrorHandler {
    /// Construct a new [`TailErrorHandler`]
    pub fn new(input: Input, follow: bool, retry: bool) -> Self {
        Self {
            printer: TailErrorPrinter::new(input.display_name.as_str()),
            input,
            follow,
            retry,
        }
    }

    /// Construct a new [`TailErrorHandler`] from an [`Input`] and the [`Observer`].
    pub fn from(input: Input, observer: &Observer) -> Self {
        Self::new(input, observer.follow.is_some(), observer.retry)
    }

    /// Handle the [`TailError`].
    ///
    /// Sets the exit code to `1`, prints the correct error message based on the context in which an
    /// [`io::Error`] happened
    pub fn handle(
        &self,
        error: &TailError,
        file: Option<File>,
        observer: &mut Observer,
    ) -> UResult<()> {
        set_exit_code(1);
        self.printer.print_error(error);

        // TODO: what about bad stdin??
        match error {
            TailError::Write(_) => return Err(USimpleError::new(1, "")),
            TailError::Open(_) if self.input.is_stdin() => {}
            TailError::Open(_) => {
                // TODO: register bad file should take an Option<Path> instead of a path
                observer.add_bad_path(
                    &self.input.path().unwrap(),
                    &self.input.display_name,
                    false,
                )?;
            }
            TailError::Stat(_) | TailError::Read(_)
                if cfg!(windows) && Self::is_directory_error(error) =>
            {
                self.handle_untailable();

                observer.add_untailable(
                    &self.input.path().unwrap(),
                    &self.input.display_name,
                    false,
                )?;
            }
            TailError::Stat(_) | TailError::Read(_) if self.input.path().is_some() => {
                // unwrap is safe here because only TailError::Open doesn't produce a File
                let file = file.unwrap();
                match file.metadata() {
                    Ok(meta) if meta.is_tailable() => {
                        observer.add_bad_path(
                            &self.input.path().unwrap(),
                            &self.input.display_name,
                            false,
                        )?;
                    }
                    Ok(_) | Err(_) => {
                        self.handle_untailable();

                        observer.add_untailable(
                            &self.input.path().unwrap(),
                            &self.input.display_name,
                            false,
                        )?;
                    }
                }
            }
            // FIXME: We're here on windows if the input was stdin, because we don't have a path on
            // windows for stdin. Currently, the follow module depends on having a path for all
            // inputs. This should be fixed. For now, we print the error message but don't follow
            // the input and effectively do nothing.
            TailError::Stat(_) | TailError::Read(_) => {}
        }

        Ok(())
    }

    /// Return true if the [`io::Error`] is a `IsADirectory` error kind.
    pub fn is_directory_error(error: &TailError) -> bool {
        match error {
            TailError::Stat(error)
            | TailError::Open(error)
            | TailError::Read(error)
            | TailError::Write(error) => {
                let os_error = error.raw_os_error();
                let message = error.to_string();

                #[cfg(unix)]
                return os_error.map_or(false, |n| n == 21)
                    || message.contains(text::IS_A_DIRECTORY);

                // This is the closest to a unix directory error I could find here
                // https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
                // in the rust stdlib `library/std/src/sys/windows/mod.rs` and `library/std/src/sys/windows/c/errors.rs`
                // os_error == 336 maps to ErrorKind::IsADirectory (which is not stable
                // at the time of writing this with rustc 1.60.0)
                #[cfg(windows)]
                return os_error.map_or(false, |n| n == 336)
                    || message.contains("An operation is not supported on a directory");
            }
        }
    }

    /// Depending on the follow mode and if `--retry` was given print the error message or not.
    ///
    /// No exit codes are set or any check is executed to ensure that the input is `tailable` or
    /// not. This method only prints the error message.
    pub fn handle_untailable(&self) {
        if self.follow {
            let msg = if self.retry {
                ""
            } else {
                "; giving up on this name"
            };
            self.printer.print_message(format!(
                "{}: cannot follow end of this type of file{}",
                self.input.display_name, msg,
            ));
        }
    }
}
