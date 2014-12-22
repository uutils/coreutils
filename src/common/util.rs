/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![macro_escape]

extern crate libc;

#[macro_export]
macro_rules! show_error(
    ($($args:expr),+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: error: ", ::NAME);
        pipe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:expr),+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: warning: ", ::NAME);
        pipe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
);

#[macro_export]
macro_rules! show_info(
    ($($args:expr),+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: ", ::NAME);
        pipe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
);

#[macro_export]
macro_rules! eprint(
    ($($args:expr),+) => (pipe_write!(&mut ::std::io::stderr(), $($args),+))
);

#[macro_export]
macro_rules! eprintln(
    ($($args:expr),+) => (pipe_writeln!(&mut ::std::io::stderr(), $($args),+))
);

#[macro_export]
macro_rules! crash(
    ($exitcode:expr, $($args:expr),+) => ({
        show_error!($($args),+);
        unsafe { ::util::libc::exit($exitcode as ::util::libc::c_int); }
    })
);

#[macro_export]
macro_rules! exit(
    ($exitcode:expr) => ({
        unsafe { ::util::libc::exit($exitcode); }
    })
);

#[macro_export]
macro_rules! crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!($exitcode, "{}", f.to_string())
        }
    )
);

#[macro_export]
macro_rules! return_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => {
                show_error!("{}", f);
                return $exitcode;
            }
        }
    )
);

// XXX: should the pipe_* macros return an Err just to show the write failed?

#[macro_export]
macro_rules! pipe_print(
    ($($args:expr),+) => (
        match write!(&mut ::std::io::stdout(), $($args),+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind == ::std::io::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_println(
    ($($args:expr),+) => (
        match writeln!(&mut ::std::io::stdout(), $($args),+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind == ::std::io::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_write(
    ($fd:expr, $($args:expr),+) => (
        match write!($fd, $($args),+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind == ::std::io::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_writeln(
    ($fd:expr, $($args:expr),+) => (
        match writeln!($fd, $($args),+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind == ::std::io::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! safe_write(
    ($fd:expr, $($args:expr),+) => (
        match write!($fd, $($args),+) {
            Ok(_) => {}
            Err(f) => panic!(f.to_string())
        }
    )
);

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:expr),+) => (
        match writeln!($fd, $($args),+) {
            Ok(_) => {}
            Err(f) => panic!(f.to_string())
        }
    )
);

#[macro_export]
macro_rules! safe_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!(1, "{}", f.to_string())
        }
    )
);
