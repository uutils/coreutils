//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) execvp SIGHUP cproc vprocmgr cstrs homeout

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use libc::{c_char, dup2, execvp, signal};
use libc::{SIGHUP, SIG_IGN};
use std::env;
use std::ffi::CString;
use std::fmt::{Display, Formatter};
use std::fs::{File, OpenOptions};
use std::io::Error;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{set_exit_code, UError, UResult};
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "Run COMMAND ignoring hangup signals.";
static LONG_HELP: &str = "
If standard input is terminal, it'll be replaced with /dev/null.
If standard output is terminal, it'll be appended to nohup.out instead,
or $HOME/nohup.out, if nohup.out open failed.
If standard error is terminal, it'll be redirected to stdout.
";
const USAGE: &str = "\
    {} COMMAND [ARG]...
    {} FLAG";
static NOHUP_OUT: &str = "nohup.out";
// exit codes that match the GNU implementation
static EXIT_CANCELED: i32 = 125;
static EXIT_CANNOT_INVOKE: i32 = 126;
static EXIT_ENOENT: i32 = 127;
static POSIX_NOHUP_FAILURE: i32 = 127;

mod options {
    pub const CMD: &str = "cmd";
}

#[derive(Debug)]
enum NohupError {
    CannotDetach,
    CannotReplace(&'static str, std::io::Error),
    OpenFailed(i32, std::io::Error),
    OpenFailed2(i32, std::io::Error, String, std::io::Error),
}

impl std::error::Error for NohupError {}

impl UError for NohupError {
    fn code(&self) -> i32 {
        match self {
            NohupError::OpenFailed(code, _) | NohupError::OpenFailed2(code, _, _, _) => *code,
            _ => 2,
        }
    }
}

impl Display for NohupError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            NohupError::CannotDetach => write!(f, "Cannot detach from console"),
            NohupError::CannotReplace(s, e) => write!(f, "Cannot replace {}: {}", s, e),
            NohupError::OpenFailed(_, e) => {
                write!(f, "failed to open {}: {}", NOHUP_OUT.quote(), e)
            }
            NohupError::OpenFailed2(_, e1, s, e2) => write!(
                f,
                "failed to open {}: {}\nfailed to open {}: {}",
                NOHUP_OUT.quote(),
                e1,
                s.quote(),
                e2
            ),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    replace_fds()?;

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { !_vprocmgr_detach_from_console(0).is_null() } {
        return Err(NohupError::CannotDetach.into());
    };

    let cstrs: Vec<CString> = matches
        .values_of(options::CMD)
        .unwrap()
        .map(|x| CString::new(x.as_bytes()).unwrap())
        .collect();
    let mut args: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(std::ptr::null());

    let ret = unsafe { execvp(args[0], args.as_mut_ptr()) };
    match ret {
        libc::ENOENT => set_exit_code(EXIT_ENOENT),
        _ => set_exit_code(EXIT_CANNOT_INVOKE),
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(options::CMD)
                .hide(true)
                .required(true)
                .multiple_occurrences(true),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}

fn replace_fds() -> UResult<()> {
    if atty::is(atty::Stream::Stdin) {
        let new_stdin = File::open(Path::new("/dev/null"))
            .map_err(|e| NohupError::CannotReplace("STDIN", e))?;
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            return Err(NohupError::CannotReplace("STDIN", Error::last_os_error()).into());
        }
    }

    if atty::is(atty::Stream::Stdout) {
        let new_stdout = find_stdout()?;
        let fd = new_stdout.as_raw_fd();

        if unsafe { dup2(fd, 1) } != 1 {
            return Err(NohupError::CannotReplace("STDOUT", Error::last_os_error()).into());
        }
    }

    if atty::is(atty::Stream::Stderr) && unsafe { dup2(1, 2) } != 2 {
        return Err(NohupError::CannotReplace("STDERR", Error::last_os_error()).into());
    }
    Ok(())
}

fn find_stdout() -> UResult<File> {
    let internal_failure_code = match std::env::var("POSIXLY_CORRECT") {
        Ok(_) => POSIX_NOHUP_FAILURE,
        Err(_) => EXIT_CANCELED,
    };

    match OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(Path::new(NOHUP_OUT))
    {
        Ok(t) => {
            show_error!(
                "ignoring input and appending output to {}",
                NOHUP_OUT.quote()
            );
            Ok(t)
        }
        Err(e1) => {
            let home = match env::var("HOME") {
                Err(_) => return Err(NohupError::OpenFailed(internal_failure_code, e1).into()),
                Ok(h) => h,
            };
            let mut homeout = PathBuf::from(home);
            homeout.push(NOHUP_OUT);
            let homeout_str = homeout.to_str().unwrap();
            match OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&homeout)
            {
                Ok(t) => {
                    show_error!(
                        "ignoring input and appending output to {}",
                        homeout_str.quote()
                    );
                    Ok(t)
                }
                Err(e2) => Err(NohupError::OpenFailed2(
                    internal_failure_code,
                    e1,
                    homeout_str.to_string(),
                    e2,
                )
                .into()),
            }
        }
    }
}

#[cfg(target_vendor = "apple")]
extern "C" {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int {
    std::ptr::null()
}
