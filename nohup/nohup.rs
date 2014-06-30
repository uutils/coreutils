#![crate_id = "nohup#1.0.0"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]
extern crate getopts;
extern crate libc;
extern crate native;

use getopts::{optflag, getopts, usage};
use std::io::stdio::{stdin_raw, stdout_raw, stderr_raw};
use std::rt::rtio::{Open, Read, Append, Write};
use libc::funcs::posix88::unistd::{dup2, execvp};
use libc::consts::os::posix88::SIGHUP;
use libc::funcs::posix01::signal::signal;
use libc::consts::os::posix01::SIG_IGN;

#[path = "../common/util.rs"] mod util;
#[path = "../common/c_types.rs"] mod c_types;

static NAME: &'static str = "nohup";
static VERSION: &'static str = "1.0.0";

#[cfg(target_os = "macos")]
extern {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(target_os = "macos")]
fn rewind_stdout<T: std::rt::rtio::RtioFileStream>(s: &mut T) {
    match s.seek(0, std::rt::rtio::SeekEnd) {
        Ok(_) => {}
        Err(f) => crash!(1, "{}", f.detail.unwrap())
    }
}

#[cfg(target_os = "linux")]
#[cfg(target_os = "freebsd")]
fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int { std::ptr::null() }

#[cfg(target_os = "linux")]
#[cfg(target_os = "freebsd")]
fn rewind_stdout<T: std::rt::rtio::RtioFileStream>(_: &mut T) {}

pub fn uumain(args: Vec<String>) -> int {
    let program = args.get(0);

    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
    ];

    let opts = match getopts(args.tail(), options) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(program.as_slice(), options);
            return 1
        }
    };

    if opts.opt_present("V") { version(); return 0 }
    if opts.opt_present("h") { show_usage(program.as_slice(), options); return 0 }

    if opts.free.len() == 0 {
        show_error!("Missing operand: COMMAND");
        println!("Try `{:s} --help` for more information.", program.as_slice());
        return 1
    }
    replace_fds();

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { _vprocmgr_detach_from_console(0) } != std::ptr::null() { crash!(2, "Cannot detach from console")};

    unsafe {
        // we ignore the memory leak here because it doesn't matter anymore
        let executable = opts.free.get(0).as_slice().to_c_str().unwrap();
        let mut args: Vec<*const i8> = opts.free.iter().map(|x| x.to_c_str().unwrap()).collect();
        args.push(std::ptr::null());
        execvp(executable as *const libc::c_char, args.as_ptr() as *mut *const libc::c_char) as int
    }
}

fn replace_fds() {
    let replace_stdin = stdin_raw().isatty();
    let replace_stdout = stdout_raw().isatty();
    let replace_stderr = stderr_raw().isatty();

    if replace_stdin {
        let devnull = "/dev/null".to_c_str();
        let new_stdin = match native::io::file::open(&devnull, Open, Read) {
            Ok(t) => t,
            Err(_) => {
                let e = std::io::IoError::last_error();
                crash!(2, "Cannot replace STDIN: {}", e)
            }
        };
        if unsafe { dup2(new_stdin.fd(), 0) } != 0 {
            crash!(2, "Cannot replace STDIN: {}", std::io::IoError::last_error())
        }
    }

    if replace_stdout {
        let mut new_stdout = find_stdout();

        rewind_stdout(&mut new_stdout);

        if unsafe { dup2(new_stdout.fd(), 1) } != 1 {
            crash!(2, "Cannot replace STDOUT: {}", std::io::IoError::last_error())
        }
    }

    if replace_stderr {
        if unsafe { dup2(1, 2) } != 2 {
            crash!(2, "Cannot replace STDERR: {}", std::io::IoError::last_error())
        }
    }
}

fn find_stdout() -> native::io::file::FileDesc {
    let localout = "nohup.out".to_c_str();
    match native::io::file::open(&localout, Append, Write) {
        Ok(t) => {
            show_warning!("Output is redirected to: nohup.out");
            t
        },
        Err(_) => {
            let e = std::io::IoError::last_error();
            let home = match std::os::getenv("HOME") {
                None => crash!(2, "Cannot replace STDOUT: {}", e),
                Some(h) => h
            };
            let mut homeoutpath = Path::new(home);
            homeoutpath.push("nohup.out");
            let homeout = homeoutpath.to_c_str();
            match native::io::file::open(&homeout, Append, Write) {
                Ok(t) => {
                    show_warning!("Output is redirected to: {}", homeoutpath.display());
                    t
                },
                Err(_) => {
                    let e = std::io::IoError::last_error();
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
    println!("  {:s} COMMAND [ARG]…", program);
    println!("  {:s} OPTION", program);
    println!("");
    print!("{:s}", usage(
            "Run COMMAND ignoring hangup signals.\n\
            If standard input is terminal, it'll be replaced with /dev/null.\n\
            If standard output is terminal, it'll be appended to nohup.out instead, \
            or $HOME/nohup.out, if nohup.out open failed.\n\
            If standard error is terminal, it'll be redirected to stdout.", options)
    );
}
