/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_export]
macro_rules! executable(
    () => ({
        let module = module_path!();
        if &module[0..3] == "uu_" {
            &module[3..]
        } else {
            module
        }
    })
);

#[macro_export]
macro_rules! show_error(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: error: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: warning: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! show_info(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! eprint(
    ($($args:tt)+) => (pipe_write!(&mut ::std::io::stderr(), $($args)+))
);

#[macro_export]
macro_rules! eprintln(
    ($($args:tt)+) => (pipe_writeln!(&mut ::std::io::stderr(), $($args)+))
);

#[macro_export]
macro_rules! crash(
    ($exitcode:expr, $($args:tt)+) => ({
        show_error!($($args)+);
        ::std::process::exit($exitcode)
    })
);

#[macro_export]
macro_rules! exit(
    ($exitcode:expr) => ({
        ::std::process::exit($exitcode)
    })
);

#[macro_export]
macro_rules! crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!($exitcode, "{}", f),
        }
    )
);

#[macro_export]
macro_rules! pipe_crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(_) => (),
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    ()
                } else {
                    crash!($exitcode, "{}", f)
                }
            },
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
    ($($args:tt)+) => (
        match write!(&mut ::std::io::stdout(), $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
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
    ($($args:tt)+) => (
        match writeln!(&mut ::std::io::stdout(), $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
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
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
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
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_flush(
    () => (
        match ::std::io::stdout().flush() {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    );
    ($fd:expr) => (
        match $fd.flush() {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
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
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!(f.to_string())
        }
    )
);

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
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
