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

#[macro_export]
macro_rules! show(
    ($err:expr) => ({
        let e = $err;
        $crate::error::set_exit_code(e.code());
        eprintln!("{}: {}", $crate::util_name(), e);
    })
);

#[macro_export]
macro_rules! show_if_err(
    ($res:expr) => ({
        if let Err(e) = $res {
            show!(e);
        }
    })
);

/// Show an error to stderr in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::util_name());
        eprintln!($($args)+);
    })
);

/// Show a warning to stderr in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_error_custom_description (
    ($err:expr,$($args:tt)+) => ({
        eprint!("{}: {}: ", $crate::util_name(), $err);
        eprintln!($($args)+);
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        eprint!("{}: warning: ", $crate::util_name());
        eprintln!($($args)+);
    })
);

/// Show a bad invocation help message in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_usage_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::util_name());
        eprintln!($($args)+);
        eprintln!("Try '{} --help' for more information.", $crate::execution_phrase());
    })
);

//====

/// Calls `exit()` with the provided exit code.
#[macro_export]
macro_rules! exit(
    ($exit_code:expr) => ({
        ::std::process::exit($exit_code)
    })
);

/// Display the provided error message, then `exit()` with the provided exit code
#[macro_export]
macro_rules! crash(
    ($exit_code:expr, $($args:tt)+) => ({
        $crate::show_error!($($args)+);
        $crate::exit!($exit_code)
    })
);

/// Unwraps the Result. Instead of panicking, it exists the program with the
/// provided exit code.
#[macro_export]
macro_rules! crash_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => $crate::crash!($exit_code, "{}", f),
        }
    )
);

//====

#[macro_export]
macro_rules! safe_write(
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!("{}", f)
        }
    )
);

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!("{}", f)
        }
    )
);

/// Unwraps the Result. Instead of panicking, it exists the program with exit
/// code 1.
#[macro_export]
macro_rules! safe_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => $crate::crash!(1, "{}", f.to_string())
        }
    )
);
