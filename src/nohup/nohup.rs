#![crate_name = "nohup"]
#![feature(collections, core, io, libc, os, path, rustc_private, std_misc)]

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

use getopts::{optflag, getopts, usage};
use std::ffi::CString;
use std::old_io::stdio::{stdin_raw, stdout_raw, stderr_raw};
use std::old_io::{File, Open, Read, Append, Write};
use std::os::unix::prelude::*;
use libc::c_char;
use libc::funcs::posix88::unistd::{dup2, execvp};
use libc::consts::os::posix88::SIGHUP;
use libc::funcs::posix01::signal::signal;
use libc::consts::os::posix01::SIG_IGN;

#[path = "../common/util.rs"] #[macro_use] mod util;
#[path = "../common/c_types.rs"] mod c_types;

static NAME: &'static str = "nohup";
static VERSION: &'static str = "1.0.0";

#[cfg(target_os = "macos")]
extern {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int { std::ptr::null() }

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0];

    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
    ];

    let opts = match getopts(args.tail(), &options) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(program.as_slice(), &options);
            return 1
        }
    };

    if opts.opt_present("V") { version(); return 0 }
    if opts.opt_present("h") { show_usage(program.as_slice(), &options); return 0 }

    if opts.free.len() == 0 {
        show_error!("Missing operand: COMMAND");
        println!("Try `{} --help` for more information.", program.as_slice());
        return 1
    }
    replace_fds();

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { _vprocmgr_detach_from_console(0) } != std::ptr::null() { crash!(2, "Cannot detach from console")};

    let cstrs : Vec<CString> = opts.free.iter().map(|x| CString::from_slice(x.as_bytes())).collect();
    let mut args : Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(std::ptr::null());
    unsafe { execvp(args[0], args.as_mut_ptr()) as isize }
}

fn replace_fds() {
    let replace_stdin = stdin_raw().isatty();
    let replace_stdout = stdout_raw().isatty();
    let replace_stderr = stderr_raw().isatty();

    if replace_stdin {
        let new_stdin = match File::open_mode(&Path::new("/dev/null"), Open, Read) {
            Ok(t) => t,
            Err(e) => {
                crash!(2, "Cannot replace STDIN: {}", e)
            }
        };
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            crash!(2, "Cannot replace STDIN: {}", std::old_io::IoError::last_error())
        }
    }

    if replace_stdout {
        let new_stdout = find_stdout();
        let fd = new_stdout.as_raw_fd();

        if unsafe { dup2(fd, 1) } != 1 {
            crash!(2, "Cannot replace STDOUT: {}", std::old_io::IoError::last_error())
        }
    }

    if replace_stderr {
        if unsafe { dup2(1, 2) } != 2 {
            crash!(2, "Cannot replace STDERR: {}", std::old_io::IoError::last_error())
        }
    }
}

fn find_stdout() -> File {
    match File::open_mode(&Path::new("nohup.out"), Append, Write) {
        Ok(t) => {
            show_warning!("Output is redirected to: nohup.out");
            t
        },
        Err(e) => {
            let home = match std::os::getenv("HOME") {
                None => crash!(2, "Cannot replace STDOUT: {}", e),
                Some(h) => h
            };
            let mut homeout = Path::new(home);
            homeout.push("nohup.out");
            match File::open_mode(&homeout, Append, Write) {
                Ok(t) => {
                    show_warning!("Output is redirected to: {}", homeout.display());
                    t
                },
                Err(e) => {
                    crash!(2, "Cannot replace STDOUT: {}", e)
                }
            }
        }
    }
}

fn version() {
    println!("{} v{}", NAME, VERSION)
}

fn show_usage(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("Usage:");
    println!("  {} COMMAND [ARG]…", program);
    println!("  {} OPTION", program);
    println!("");
    print!("{}", usage(
            "Run COMMAND ignoring hangup signals.\n\
            If standard input is terminal, it'll be replaced with /dev/null.\n\
            If standard output is terminal, it'll be appended to nohup.out instead, \
            or $HOME/nohup.out, if nohup.out open failed.\n\
            If standard error is terminal, it'll be redirected to stdout.", options)
    );
}
