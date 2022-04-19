//! All utils return exit with an exit code. Usually, the following scheme is used:
//! * `0`: succeeded
//! * `1`: minor problems
//! * `2`: major problems
//!
//! This module provides types to reconcile these exit codes with idiomatic Rust error
//! handling. This has a couple advantages over manually using [`std::process::exit`]:
//! 1. It enables the use of `?`, `map_err`, `unwrap_or`, etc. in `uumain`.
//! 1. It encourages the use of [`UResult`]/[`Result`] in functions in the utils.
//! 1. The error messages are largely standardized across utils.
//! 1. Standardized error messages can be created from external result types
//!    (i.e. [`std::io::Result`] & `clap::ClapResult`).
//! 1. [`set_exit_code`] takes away the burden of manually tracking exit codes for non-fatal errors.
//!
//! # Usage
//! The signature of a typical util should be:
//! ```ignore
//! fn uumain(args: impl uucore::Args) -> UResult<()> {
//!     ...
//! }
//! ```
//! [`UResult`] is a simple wrapper around [`Result`] with a custom error trait: [`UError`]. The
//! most important difference with types implementing [`std::error::Error`] is that [`UError`]s
//! can specify the exit code of the program when they are returned from `uumain`:
//! * When `Ok` is returned, the code set with [`set_exit_code`] is used as exit code. If
//!   [`set_exit_code`] was not used, then `0` is used.
//! * When `Err` is returned, the code corresponding with the error is used as exit code and the
//! error message is displayed.
//!
//! Additionally, the errors can be displayed manually with the [`show`] and [`show_if_err`] macros:
//! ```ignore
//! let res = Err(USimpleError::new(1, "Error!!"));
//! show_if_err!(res);
//! // or
//! if let Err(e) = res {
//!    show!(e);
//! }
//! ```
//!
//! **Note**: The [`show`] and [`show_if_err`] macros set the exit code of the program using
//! [`set_exit_code`]. See the documentation on that function for more information.
//!
//! # Guidelines
//! * Use error types from `uucore` where possible.
//! * Add error types to `uucore` if an error appears in multiple utils.
//! * Prefer proper custom error types over [`ExitCode`] and [`USimpleError`].
//! * [`USimpleError`] may be used in small utils with simple error handling.
//! * Using [`ExitCode`] is not recommended but can be useful for converting utils to use
//!   [`UResult`].

// spell-checker:ignore uioerror

use clap;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    sync::atomic::{AtomicI32, Ordering},
};

static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

/// Get the last exit code set with [`set_exit_code`].
/// The default value is `0`.
pub fn get_exit_code() -> i32 {
    EXIT_CODE.load(Ordering::SeqCst)
}

/// Set the exit code for the program if `uumain` returns `Ok(())`.
///
/// This function is most useful for non-fatal errors, for example when applying an operation to
/// multiple files:
/// ```ignore
/// use uucore::error::{UResult, set_exit_code};
///
/// fn uumain(args: impl uucore::Args) -> UResult<()> {
///     ...
///     for file in files {
///         let res = some_operation_that_might_fail(file);
///         match res {
///             Ok() => {},
///             Err(_) => set_exit_code(1),
///         }
///     }
///     Ok(()) // If any of the operations failed, 1 is returned.
/// }
/// ```
pub fn set_exit_code(code: i32) {
    EXIT_CODE.store(code, Ordering::SeqCst);
}

/// Result type that should be returned by all utils.
pub type UResult<T> = Result<T, Box<dyn UError>>;

/// Custom errors defined by the utils and `uucore`.
///
/// All errors should implement [`std::error::Error`], [`std::fmt::Display`] and
/// [`std::fmt::Debug`] and have an additional `code` method that specifies the
/// exit code of the program if the error is returned from `uumain`.
///
/// An example of a custom error from `ls`:
///
/// ```
/// use uucore::{
///     display::Quotable,
///     error::{UError, UResult}
/// };
/// use std::{
///     error::Error,
///     fmt::{Display, Debug},
///     path::PathBuf
/// };
///
/// #[derive(Debug)]
/// enum LsError {
///     InvalidLineWidth(String),
///     NoMetadata(PathBuf),
/// }
///
/// impl UError for LsError {
///     fn code(&self) -> i32 {
///         match self {
///             LsError::InvalidLineWidth(_) => 2,
///             LsError::NoMetadata(_) => 1,
///         }
///     }
/// }
///
/// impl Error for LsError {}
///
/// impl Display for LsError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             LsError::InvalidLineWidth(s) => write!(f, "invalid line width: {}", s.quote()),
///             LsError::NoMetadata(p) => write!(f, "could not open file: {}", p.quote()),
///         }
///     }
/// }
/// ```
///
/// The main routine would look like this:
///
/// ```ignore
/// #[uucore::main]
/// pub fn uumain(args: impl uucore::Args) -> UResult<()> {
///     // Perform computations here ...
///     return Err(LsError::InvalidLineWidth(String::from("test")).into())
/// }
/// ```
///
/// The call to `into()` is required to convert the `LsError` to
/// [`Box<dyn UError>`]. The implementation for `From` is provided automatically.
///
/// A crate like [`quick_error`](https://crates.io/crates/quick-error) might
/// also be used, but will still require an `impl` for the `code` method.
pub trait UError: Error + Send {
    /// Error code of a custom error.
    ///
    /// Set a return value for each variant of an enum-type to associate an
    /// error code (which is returned to the system shell) with an error
    /// variant.
    ///
    /// # Example
    ///
    /// ```
    /// use uucore::{
    ///     display::Quotable,
    ///     error::UError
    /// };
    /// use std::{
    ///     error::Error,
    ///     fmt::{Display, Debug},
    ///     path::PathBuf
    /// };
    ///
    /// #[derive(Debug)]
    /// enum MyError {
    ///     Foo(String),
    ///     Bar(PathBuf),
    ///     Bing(),
    /// }
    ///
    /// impl UError for MyError {
    ///     fn code(&self) -> i32 {
    ///         match self {
    ///             MyError::Foo(_) => 2,
    ///             // All other errors yield the same error code, there's no
    ///             // need to list them explicitly.
    ///             _ => 1,
    ///         }
    ///     }
    /// }
    ///
    /// impl Error for MyError {}
    ///
    /// impl Display for MyError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         use MyError as ME;
    ///         match self {
    ///             ME::Foo(s) => write!(f, "Unknown Foo: {}", s.quote()),
    ///             ME::Bar(p) => write!(f, "Couldn't find Bar: {}", p.quote()),
    ///             ME::Bing() => write!(f, "Exterminate!"),
    ///         }
    ///     }
    /// }
    /// ```
    fn code(&self) -> i32 {
        1
    }

    /// Print usage help to a custom error.
    ///
    /// Return true or false to control whether a short usage help is printed
    /// below the error message. The usage help is in the format: "Try `{name}
    /// --help` for more information." and printed only if `true` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use uucore::{
    ///     display::Quotable,
    ///     error::UError
    /// };
    /// use std::{
    ///     error::Error,
    ///     fmt::{Display, Debug},
    ///     path::PathBuf
    /// };
    ///
    /// #[derive(Debug)]
    /// enum MyError {
    ///     Foo(String),
    ///     Bar(PathBuf),
    ///     Bing(),
    /// }
    ///
    /// impl UError for MyError {
    ///     fn usage(&self) -> bool {
    ///         match self {
    ///             // This will have a short usage help appended
    ///             MyError::Bar(_) => true,
    ///             // These matches won't have a short usage help appended
    ///             _ => false,
    ///         }
    ///     }
    /// }
    ///
    /// impl Error for MyError {}
    ///
    /// impl Display for MyError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         use MyError as ME;
    ///         match self {
    ///             ME::Foo(s) => write!(f, "Unknown Foo: {}", s.quote()),
    ///             ME::Bar(p) => write!(f, "Couldn't find Bar: {}", p.quote()),
    ///             ME::Bing() => write!(f, "Exterminate!"),
    ///         }
    ///     }
    /// }
    /// ```
    fn usage(&self) -> bool {
        false
    }
}

impl<T> From<T> for Box<dyn UError>
where
    T: UError + 'static,
{
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

/// A simple error type with an exit code and a message that implements [`UError`].
///
/// ```
/// use uucore::error::{UResult, USimpleError};
/// let err = USimpleError { code: 1, message: "error!".into()};
/// let res: UResult<()> = Err(err.into());
/// // or using the `new` method:
/// let res: UResult<()> = Err(USimpleError::new(1, "error!"));
/// ```
#[derive(Debug)]
pub struct USimpleError {
    pub code: i32,
    pub message: String,
}

impl USimpleError {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(code: i32, message: S) -> Box<dyn UError> {
        Box::new(Self {
            code,
            message: message.into(),
        })
    }
}

impl Error for USimpleError {}

impl Display for USimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.message.fmt(f)
    }
}

impl UError for USimpleError {
    fn code(&self) -> i32 {
        self.code
    }
}

#[derive(Debug)]
pub struct UUsageError {
    pub code: i32,
    pub message: String,
}

impl UUsageError {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(code: i32, message: S) -> Box<dyn UError> {
        Box::new(Self {
            code,
            message: message.into(),
        })
    }
}

impl Error for UUsageError {}

impl Display for UUsageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.message.fmt(f)
    }
}

impl UError for UUsageError {
    fn code(&self) -> i32 {
        self.code
    }

    fn usage(&self) -> bool {
        true
    }
}

/// Wrapper type around [`std::io::Error`].
///
/// The messages displayed by [`UIoError`] should match the error messages displayed by GNU
/// coreutils.
///
/// There are two ways to construct this type: with [`UIoError::new`] or by calling the
/// [`FromIo::map_err_context`] method on a [`std::io::Result`] or [`std::io::Error`].
/// ```
/// use uucore::{
///     display::Quotable,
///     error::{FromIo, UResult, UIoError, UError}
/// };
/// use std::fs::File;
/// use std::path::Path;
/// let path = Path::new("test.txt");
///
/// // Manual construction
/// let e: Box<dyn UError> = UIoError::new(
///     std::io::ErrorKind::NotFound,
///     format!("cannot access {}", path.quote())
/// );
/// let res: UResult<()> = Err(e.into());
///
/// // Converting from an `std::io::Error`.
/// let res: UResult<File> = File::open(path).map_err_context(|| format!("cannot access {}", path.quote()));
/// ```
#[derive(Debug)]
pub struct UIoError {
    context: Option<String>,
    inner: std::io::Error,
}

impl UIoError {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(kind: std::io::ErrorKind, context: S) -> Box<dyn UError> {
        Box::new(Self {
            context: Some(context.into()),
            inner: kind.into(),
        })
    }
}

impl UError for UIoError {}

impl Error for UIoError {}

impl Display for UIoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        use std::io::ErrorKind::*;

        let mut message;
        let message = if self.inner.raw_os_error().is_some() {
            // These are errors that come directly from the OS.
            // We want to normalize their messages across systems,
            // and we want to strip the "(os error X)" suffix.
            match self.inner.kind() {
                NotFound => "No such file or directory",
                PermissionDenied => "Permission denied",
                ConnectionRefused => "Connection refused",
                ConnectionReset => "Connection reset",
                ConnectionAborted => "Connection aborted",
                NotConnected => "Not connected",
                AddrInUse => "Address in use",
                AddrNotAvailable => "Address not available",
                BrokenPipe => "Broken pipe",
                AlreadyExists => "Already exists",
                WouldBlock => "Would block",
                InvalidInput => "Invalid input",
                InvalidData => "Invalid data",
                TimedOut => "Timed out",
                WriteZero => "Write zero",
                Interrupted => "Interrupted",
                UnexpectedEof => "Unexpected end of file",
                _ => {
                    // TODO: When the new error variants
                    // (https://github.com/rust-lang/rust/issues/86442)
                    // are stabilized, we should add them to the match statement.
                    message = strip_errno(&self.inner);
                    capitalize(&mut message);
                    &message
                }
            }
        } else {
            // These messages don't need as much normalization, and the above
            // messages wouldn't always be a good substitute.
            // For example, ErrorKind::NotFound doesn't necessarily mean it was
            // a file that was not found.
            // There are also errors with entirely custom messages.
            message = self.inner.to_string();
            capitalize(&mut message);
            &message
        };
        if let Some(ctx) = &self.context {
            write!(f, "{}: {}", ctx, message)
        } else {
            write!(f, "{}", message)
        }
    }
}

/// Capitalize the first character of an ASCII string.
fn capitalize(text: &mut str) {
    if let Some(first) = text.get_mut(..1) {
        first.make_ascii_uppercase();
    }
}

/// Strip the trailing " (os error XX)" from io error strings.
pub fn strip_errno(err: &std::io::Error) -> String {
    let mut msg = err.to_string();
    if let Some(pos) = msg.find(" (os error ") {
        msg.truncate(pos);
    }
    msg
}

/// Enables the conversion from [`std::io::Error`] to [`UError`] and from [`std::io::Result`] to
/// [`UResult`].
pub trait FromIo<T> {
    fn map_err_context(self, context: impl FnOnce() -> String) -> T;
}

impl FromIo<Box<UIoError>> for std::io::Error {
    fn map_err_context(self, context: impl FnOnce() -> String) -> Box<UIoError> {
        Box::new(UIoError {
            context: Some((context)()),
            inner: self,
        })
    }
}

impl<T> FromIo<UResult<T>> for std::io::Result<T> {
    fn map_err_context(self, context: impl FnOnce() -> String) -> UResult<T> {
        self.map_err(|e| e.map_err_context(context) as Box<dyn UError>)
    }
}

impl FromIo<Box<UIoError>> for std::io::ErrorKind {
    fn map_err_context(self, context: impl FnOnce() -> String) -> Box<UIoError> {
        Box::new(UIoError {
            context: Some((context)()),
            inner: std::io::Error::new(self, ""),
        })
    }
}

impl From<std::io::Error> for UIoError {
    fn from(f: std::io::Error) -> Self {
        Self {
            context: None,
            inner: f,
        }
    }
}

impl From<std::io::Error> for Box<dyn UError> {
    fn from(f: std::io::Error) -> Self {
        let u_error: UIoError = f.into();
        Box::new(u_error) as Self
    }
}

/// Shorthand to construct [`UIoError`]-instances.
///
/// This macro serves as a convenience call to quickly construct instances of
/// [`UIoError`]. It takes:
///
/// - An instance of [`std::io::Error`]
/// - A `format!`-compatible string and
/// - An arbitrary number of arguments to the format string
///
/// In exactly this order. It is equivalent to the more verbose code seen in the
/// example.
///
/// # Examples
///
/// ```
/// use uucore::error::UIoError;
/// use uucore::uio_error;
///
/// let io_err = std::io::Error::new(
///     std::io::ErrorKind::PermissionDenied, "fix me please!"
/// );
///
/// let uio_err = UIoError::new(
///     io_err.kind(),
///     format!("Error code: {}", 2)
/// );
///
/// let other_uio_err = uio_error!(io_err, "Error code: {}", 2);
///
/// // prints "fix me please!: Permission denied"
/// println!("{}", uio_err);
/// // prints "Error code: 2: Permission denied"
/// println!("{}", other_uio_err);
/// ```
///
/// The [`std::fmt::Display`] impl of [`UIoError`] will then ensure that an
/// appropriate error message relating to the actual error kind of the
/// [`std::io::Error`] is appended to whatever error message is defined in
/// addition (as secondary argument).
///
/// If you want to show only the error message for the [`std::io::ErrorKind`]
/// that's contained in [`UIoError`], pass the second argument as empty string:
///
/// ```
/// use uucore::error::UIoError;
/// use uucore::uio_error;
///
/// let io_err = std::io::Error::new(
///     std::io::ErrorKind::PermissionDenied, "fix me please!"
/// );
///
/// let other_uio_err = uio_error!(io_err, "");
///
/// // prints: ": Permission denied"
/// println!("{}", other_uio_err);
/// ```
//#[macro_use]
#[macro_export]
macro_rules! uio_error(
    ($err:expr, $($args:tt)+) => ({
        UIoError::new(
            $err.kind(),
            format!($($args)+)
        )
    })
);

/// A special error type that does not print any message when returned from
/// `uumain`. Especially useful for porting utilities to using [`UResult`].
///
/// There are two ways to construct an [`ExitCode`]:
/// ```
/// use uucore::error::{ExitCode, UResult};
/// // Explicit
/// let res: UResult<()> = Err(ExitCode(1).into());
///
/// // Using into on `i32`:
/// let res: UResult<()> = Err(1.into());
/// ```
/// This type is especially useful for a trivial conversion from utils returning [`i32`] to
/// returning [`UResult`].
#[derive(Debug)]
pub struct ExitCode(pub i32);

impl ExitCode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(code: i32) -> Box<dyn UError> {
        Box::new(Self(code))
    }
}

impl Error for ExitCode {}

impl Display for ExitCode {
    fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

impl UError for ExitCode {
    fn code(&self) -> i32 {
        self.0
    }
}

impl From<i32> for Box<dyn UError> {
    fn from(i: i32) -> Self {
        ExitCode::new(i)
    }
}

/// Implementations for clap::Error
impl UError for clap::Error {
    fn code(&self) -> i32 {
        match self.kind() {
            clap::ErrorKind::DisplayHelp | clap::ErrorKind::DisplayVersion => 0,
            _ => 1,
        }
    }
}
