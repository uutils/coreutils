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
//!   - From custom messages: [`show_error!`], [`show_usage_error!`]
//! - Print warnings: [`show_warning!`]
//! - Terminate util execution
//!   - Crash program: [`crash!`], [`crash_if_err!`]

// spell-checker:ignore sourcepath targetpath

use std::sync::atomic::AtomicBool;

// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/// Whether we were called as a multicall binary ("coreutils <utility>")
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
            show!(e);
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

/// Show a warning to stderr in a similar style to GNU coreutils.
///
/// Is this really required? Used in the following locations:
///
/// ./src/uu/head/src/head.rs:12
/// ./src/uu/head/src/head.rs:424
/// ./src/uu/head/src/head.rs:427
/// ./src/uu/head/src/head.rs:430
/// ./src/uu/head/src/head.rs:453
/// ./src/uu/du/src/du.rs:339
/// ./src/uu/wc/src/wc.rs:270
/// ./src/uu/wc/src/wc.rs:273
#[macro_export]
macro_rules! show_error_custom_description (
    ($err:expr,$($args:tt)+) => ({
        eprint!("{}: {}: ", $crate::util_name(), $err);
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

/// Show a bad invocation help message in a similar style to GNU coreutils.
///
/// Takes a [`format!`]-compatible input and prepends it with the current
/// utility's name before printing to stderr.
///
/// # Examples
///
/// ```
/// # #[macro_use]
/// # extern crate uucore;
/// # fn main() {
/// // outputs <name>: Couldn't apply foo to bar
/// //         Try '<name> --help' for more information.
/// show_usage_error!("Couldn't apply {} to {}", "foo", "bar");
/// # }
/// ```
#[macro_export]
macro_rules! show_usage_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::util_name());
        eprintln!($($args)+);
        eprintln!("Try '{} --help' for more information.", $crate::execution_phrase());
    })
);

/// Display an error and [`exit!`]
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

//-- message templates

//-- message templates : (join utility sub-macros)

// used only by "cut"
#[macro_export]
macro_rules! snippet_list_join_oxford_comma {
    ($conjunction:expr, $valOne:expr, $valTwo:expr) => (
        format!("{}, {} {}", $valOne, $conjunction, $valTwo)
    );
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, $crate::snippet_list_join_oxford_comma!($conjunction, $valTwo $(, $remaining_values)*))
    );
}

// used only by "cut"
#[macro_export]
macro_rules! snippet_list_join {
    ($conjunction:expr, $valOne:expr, $valTwo:expr) => (
        format!("{} {} {}", $valOne, $conjunction, $valTwo)
    );
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, $crate::snippet_list_join_oxford_comma!($conjunction, $valTwo $(, $remaining_values)*))
    );
}

//-- message templates : invalid input

#[macro_export]
macro_rules! msg_invalid_input {
    ($reason: expr) => {
        format!("invalid input: {}", $reason)
    };
}

// -- message templates : invalid input : flag

#[macro_export]
macro_rules! msg_invalid_opt_use {
    ($about:expr, $flag:expr) => {
        $crate::msg_invalid_input!(format!("The '{}' option {}", $flag, $about))
    };
    ($about:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_input!(format!(
            "The '{}' ('{}') option {}",
            $long_flag, $short_flag, $about
        ))
    };
}

// Only used by "cut"
#[macro_export]
macro_rules! msg_opt_only_usable_if {
    ($clause:expr, $flag:expr) => {
        $crate::msg_invalid_opt_use!(format!("only usable if {}", $clause), $flag)
    };
    ($clause:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("only usable if {}", $clause),
            $long_flag,
            $short_flag
        )
    };
}

// Used only by "cut"
#[macro_export]
macro_rules! msg_opt_invalid_should_be {
    ($expects:expr, $received:expr, $flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $flag
        )
    };
    ($expects:expr, $received:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $long_flag,
            $short_flag
        )
    };
}

// -- message templates : invalid input : input combinations

// UNUSED!
#[macro_export]
macro_rules! msg_expects_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        $crate::msg_invalid_input!(format!("expects one of {}", $crate::snippet_list_join!("or", $valOne $(, $remaining_values)*)))
    );
}

// Used only by "cut"
#[macro_export]
macro_rules! msg_expects_no_more_than_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        $crate::msg_invalid_input!(format!("expects no more than one of {}", $crate::snippet_list_join!("or", $valOne $(, $remaining_values)*))) ;
    );
}
