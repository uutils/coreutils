//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) execvp SIGHUP cproc vprocmgr cstrs homeout

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, AppSettings, Arg};
use libc::{c_char, dup2, execvp, signal};
use libc::{SIGHUP, SIG_IGN};
use std::env;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::Error;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Run COMMAND ignoring hangup signals.";
static LONG_HELP: &str = "
If standard input is terminal, it'll be replaced with /dev/null.
If standard output is terminal, it'll be appended to nohup.out instead,
or $HOME/nohup.out, if nohup.out open failed.
If standard error is terminal, it'll be redirected to stdout.
";
static NOHUP_OUT: &str = "nohup.out";
// exit codes that match the GNU implementation
static EXIT_CANCELED: i32 = 125;
static EXIT_CANNOT_INVOKE: i32 = 126;
static EXIT_ENOENT: i32 = 127;
static POSIX_NOHUP_FAILURE: i32 = 127;

mod options {
    pub const CMD: &str = "cmd";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .arg(
            Arg::with_name(options::CMD)
                .hidden(true)
                .required(true)
                .multiple(true),
        )
        .setting(AppSettings::TrailingVarArg)
        .get_matches_from(args);

    replace_fds();

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { !_vprocmgr_detach_from_console(0).is_null() } {
        crash!(2, "Cannot detach from console")
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
        libc::ENOENT => EXIT_ENOENT,
        _ => EXIT_CANNOT_INVOKE,
    }
}

fn replace_fds() {
    if atty::is(atty::Stream::Stdin) {
        let new_stdin = match File::open(Path::new("/dev/null")) {
            Ok(t) => t,
            Err(e) => crash!(2, "Cannot replace STDIN: {}", e),
        };
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            crash!(2, "Cannot replace STDIN: {}", Error::last_os_error())
        }
    }

    if atty::is(atty::Stream::Stdout) {
        let new_stdout = find_stdout();
        let fd = new_stdout.as_raw_fd();

        if unsafe { dup2(fd, 1) } != 1 {
            crash!(2, "Cannot replace STDOUT: {}", Error::last_os_error())
        }
    }

    if atty::is(atty::Stream::Stderr) && unsafe { dup2(1, 2) } != 2 {
        crash!(2, "Cannot replace STDERR: {}", Error::last_os_error())
    }
}

fn find_stdout() -> File {
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
            show_error!("ignoring input and appending output to '{}'", NOHUP_OUT);
            t
        }
        Err(e1) => {
            let home = match env::var("HOME") {
                Err(_) => {
                    show_error!("failed to open '{}': {}", NOHUP_OUT, e1);
                    exit!(internal_failure_code)
                }
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
                    show_error!("ignoring input and appending output to '{}'", homeout_str);
                    t
                }
                Err(e2) => {
                    show_error!("failed to open '{}': {}", NOHUP_OUT, e1);
                    show_error!("failed to open '{}': {}", homeout_str, e2);
                    exit!(internal_failure_code)
                }
            }
        }
    }
}

fn get_usage() -> String {
    format!("{0} COMMAND [ARG]...\n    {0} FLAG", executable!())
}

#[cfg(target_vendor = "apple")]
extern "C" {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int {
    std::ptr::null()
}
