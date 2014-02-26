/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_escape];

pub fn program_name () -> &'static str { ::NAME }

#[macro_export]
macro_rules! show_error(
    ($exitcode:expr, $($args:expr),+) => ({
        ::std::os::set_exit_status($exitcode);
        safe_write!(&mut ::std::io::stderr(), "{}: error: ", ::util::program_name());
        safe_writeln!(&mut ::std::io::stderr(), $($args),+);
    })
)

#[macro_export]
macro_rules! crash(
    ($exitcode:expr, $($args:expr),+) => ({
        show_error!($exitcode, $($args),+);
        unsafe { ::std::libc::exit($exitcode); }
    })
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
