#![crate_name = "uu_nohup"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use libc::{c_char, signal, dup2, execvp};
use libc::{SIG_IGN, SIGHUP};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Error, Write};
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use std::env;
use uucore::fs::{is_stderr_interactive, is_stdin_interactive, is_stdout_interactive};

static NAME: &'static str = "nohup";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(target_os = "macos")]
extern {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int { std::ptr::null() }

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "Show help and exit");
    opts.optflag("V", "version", "Show version and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(&opts);
            return 1
        }
    };

    if matches.opt_present("V") { println!("{} {}", NAME, VERSION); return 0 }
    if matches.opt_present("h") { show_usage(&opts); return 0 }

    if matches.free.is_empty() {
        show_error!("Missing operand: COMMAND");
        println!("Try `{} --help` for more information.", NAME);
        return 1
    }
    replace_fds();

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { _vprocmgr_detach_from_console(0) } != std::ptr::null() { crash!(2, "Cannot detach from console")};

    let cstrs: Vec<CString> = matches.free.iter().map(|x| CString::new(x.as_bytes()).unwrap()).collect();
    let mut args: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(std::ptr::null());
    unsafe { execvp(args[0], args.as_mut_ptr())}
}

fn replace_fds() {
    if is_stdin_interactive() {
        let new_stdin = match File::open(Path::new("/dev/null")) {
            Ok(t) => t,
            Err(e) => {
                crash!(2, "Cannot replace STDIN: {}", e)
            }
        };
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            crash!(2, "Cannot replace STDIN: {}", Error::last_os_error())
        }
    }

    if is_stdout_interactive() {
        let new_stdout = find_stdout();
        let fd = new_stdout.as_raw_fd();

        if unsafe { dup2(fd, 1) } != 1 {
            crash!(2, "Cannot replace STDOUT: {}", Error::last_os_error())
        }
    }

    if is_stderr_interactive() {
        if unsafe { dup2(1, 2) } != 2 {
            crash!(2, "Cannot replace STDERR: {}", Error::last_os_error())
        }
    }
}

fn find_stdout() -> File {
    match OpenOptions::new().write(true).create(true).append(true).open(Path::new("nohup.out")) {
        Ok(t) => {
            show_warning!("Output is redirected to: nohup.out");
            t
        },
        Err(e) => {
            let home = match env::var("HOME") {
                Err(_) => crash!(2, "Cannot replace STDOUT: {}", e),
                Ok(h) => h
            };
            let mut homeout = PathBuf::from(home);
            homeout.push("nohup.out");
            match OpenOptions::new().write(true).create(true).append(true).open(&homeout) {
                Ok(t) => {
                    show_warning!("Output is redirected to: {:?}", homeout);
                    t
                },
                Err(e) => {
                    crash!(2, "Cannot replace STDOUT: {}", e)
                }
            }
        }
    }
}

fn show_usage(opts: &getopts::Options) {
    let msg = format!("{0} {1}

Usage:
  {0} COMMAND [ARG]...
  {0} OPTION

Run COMMAND ignoring hangup signals.
If standard input is terminal, it'll be replaced with /dev/null.
If standard output is terminal, it'll be appended to nohup.out instead,
or $HOME/nohup.out, if nohup.out open failed.
If standard error is terminal, it'll be redirected to stdout.", NAME, VERSION);

    print!("{}", opts.usage(&msg));
}
