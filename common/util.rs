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
        safe_write!(&mut ::std::io::stderr(), "{}: error: ", ::NAME);
        safe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
)

#[macro_export]
macro_rules! show_warning(
    ($($args:expr),+) => ({
        safe_write!(&mut ::std::io::stderr(), "{}: warning: ", ::NAME);
        safe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
)

#[macro_export]
macro_rules! crash(
    ($exitcode:expr, $($args:expr),+) => ({
        safe_write!(&mut ::std::io::stderr(), "{}: error: ", ::NAME);
        safe_writeln!(&mut ::std::io::stderr(), $($args),+);
        unsafe { self::libc::exit($exitcode as self::libc::c_int); }
    })
)


#[macro_export]
macro_rules! exit(
    ($exitcode:expr) => ({
        unsafe { self::libc::exit($exitcode); }
    })
)

#[macro_export]
macro_rules! crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!($exitcode, "{}", f.to_str())
        }
    )
)

#[macro_export]
macro_rules! safe_write(
    ($fd:expr, $($args:expr),+) => (
        match write!($fd, $($args),+) {
            Ok(_) => {}
            Err(f) => { fail!(f.to_str()); }
        }
    )
)

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:expr),+) => (
        match writeln!($fd, $($args),+) {
            Ok(_) => {}
            Err(f) => { fail!(f.to_str()); }
        }
    )
)

#[macro_export]
macro_rules! safe_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!(1, "{}", f.to_str())
        }
    )
)
