// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// TODO fix broken links
#![allow(rustdoc::broken_intra_doc_links)]
//! Macros for the uucore utilities.
//!
//! This module bundles all macros used across the uucore utilities. These
//! include macros for reporting errors in various formats, aborting program
//! execution and more.
//!
//! To make use of all macros in this module, they must be imported like so:
//!
//! ```ignore
//! #[macro_use]
//! extern crate uucore;
//! ```
//!
//! Alternatively, you can import single macros by importing them through their
//! fully qualified name like this:
//!
//! ```no_run
//! use uucore::{show, crash};
//! ```
//!
//! Here's an overview of the macros sorted by purpose
//!
//! - Print errors
//!   - From types implementing [`crate::error::UError`]: [`show!`],
//!     [`show_if_err!`]
//!   - From custom messages: [`show_error!`]
//! - Print warnings: [`show_warning!`]
//! - Terminate util execution
//!   - Crash program: [`crash!`], [`crash_if_err!`]

// spell-checker:ignore sourcepath targetpath rustdoc

use std::sync::atomic::AtomicBool;

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/// Whether we were called as a multicall binary (`coreutils <utility>`)
pub static UTILITY_IS_SECOND_ARG: AtomicBool = AtomicBool::new(false);

//====

/// Display a [`crate::error::UError`] and set global exit code.
///
/// Prints the error message contained in an [`crate::error::UError`] to stderr
/// and sets the exit code through [`crate::error::set_exit_code`]. The printed
/// error message is prepended with the calling utility's name. A call to this
/// macro will not finish program execution.
///
/// # Examples
///
/// The following example would print a message "Some error occurred" and set
/// the utility's exit code to 2.
///
/// ```
/// # #[macro_use]
/// # extern crate uucore;
///
/// use uucore::error::{self, USimpleError};
///
/// fn main() {
///     let err = USimpleError::new(2, "Some error occurred.");
///     show!(err);
///     assert_eq!(error::get_exit_code(), 2);
/// }
/// ```
///
/// If not using [`crate::error::UError`], one may achieve the same behavior
/// like this:
///
/// ```
/// # #[macro_use]
/// # extern crate uucore;
///
/// use uucore::error::set_exit_code;
///
/// fn main() {
///     set_exit_code(2);
///     show_error!("Some error occurred.");
/// }
/// ```
#[macro_export]
macro_rules! show(
    ($err:expr) => ({
        use $crate::error::UError;
        let e = $err;
        $crate::error::set_exit_code(e.code());
        eprintln!("{}: {}", $crate::util_name(), e);
    })
);

/// Display an error and set global exit code in error case.
///
/// Wraps around [`show!`] and takes a [`crate::error::UResult`] instead of a
/// [`crate::error::UError`] type. This macro invokes [`show!`] if the
/// [`crate::error::UResult`] is an `Err`-variant. This can be invoked directly
/// on the result of a function call, like in the `install` utility:
///
/// ```ignore
/// show_if_err!(copy(sourcepath, &targetpath, b));
/// ```
///
/// # Examples
///
/// ```ignore
/// # #[macro_use]
/// # extern crate uucore;
/// # use uucore::error::{UError, UIoError, UResult, USimpleError};
///
/// # fn main() {
/// let is_ok = Ok(1);
/// // This does nothing at all
/// show_if_err!(is_ok);
///
/// let is_err = Err(USimpleError::new(1, "I'm an error").into());
/// // Calls `show!` on the contained USimpleError
/// show_if_err!(is_err);
/// # }
/// ```
///
///
#[macro_export]
macro_rules! show_if_err(
    ($res:expr) => ({
        if let Err(e) = $res {
            $crate::show!(e);
        }
    })
);

/// Show an error to stderr in a similar style to GNU coreutils.
///
/// Takes a [`format!`]-like input and prints it to stderr. The output is
/// prepended with the current utility's name.
///
/// # Examples
///
/// ```
/// # #[macro_use]
/// # extern crate uucore;
/// # fn main() {
/// show_error!("Couldn't apply {} to {}", "foo", "bar");
/// # }
/// ```
#[macro_export]
macro_rules! show_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::util_name());
        eprintln!($($args)+);
    })
);

/// Print a warning message to stderr.
///
/// Takes [`format!`]-compatible input and prepends it with the current
/// utility's name and "warning: " before printing to stderr.
///
/// # Examples
///
/// ```
/// # #[macro_use]
/// # extern crate uucore;
/// # fn main() {
/// // outputs <name>: warning: Couldn't apply foo to bar
/// show_warning!("Couldn't apply {} to {}", "foo", "bar");
/// # }
/// ```
#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        eprint!("{}: warning: ", $crate::util_name());
        eprintln!($($args)+);
    })
);

#[macro_export]
macro_rules! show_warning_caps(
    ($($args:tt)+) => ({
        eprint!("{}: WARNING: ", $crate::util_name());
        eprintln!($($args)+);
    })
);

/// Display an error and [`std::process::exit`]
///
/// Displays the provided error message using [`show_error!`], then invokes
/// [`std::process::exit`] with the provided exit code.
///
/// # Examples
///
/// ```should_panic
/// # #[macro_use]
/// # extern crate uucore;
/// # fn main() {
/// // outputs <name>: Couldn't apply foo to bar
/// // and terminates execution
/// crash!(1, "Couldn't apply {} to {}", "foo", "bar");
/// # }
/// ```
#[macro_export]
macro_rules! crash(
    ($exit_code:expr, $($args:tt)+) => ({
        $crate::show_error!($($args)+);
        std::process::exit($exit_code);
    })
);

/// Unwrap a [`std::result::Result`], crashing instead of panicking.
///
/// If the result is an `Ok`-variant, returns the value contained inside. If it
/// is an `Err`-variant, invokes [`crash!`] with the formatted error instead.
///
/// # Examples
///
/// ```should_panic
/// # #[macro_use]
/// # extern crate uucore;
/// # fn main() {
/// let is_ok: Result<u32, &str> = Ok(1);
/// // Does nothing
/// crash_if_err!(1, is_ok);
///
/// let is_err: Result<u32, &str> = Err("This didn't work...");
/// // Calls `crash!`
/// crash_if_err!(1, is_err);
/// # }
/// ```
#[macro_export]
macro_rules! crash_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => $crate::crash!($exit_code, "{}", f),
        }
    )
);
