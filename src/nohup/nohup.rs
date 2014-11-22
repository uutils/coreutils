#![crate_name = "nohup"]

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

use getopts::{optflag, getopts, usage};
use std::io::stdio::{stdin_raw, stdout_raw, stderr_raw};
use std::io::{File, Open, Read, Append, Write};
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

// BEGIN CODE TO DELETE AFTER https://github.com/rust-lang/rust/issues/18897 is fixed
struct HackyFile {
    pub fd: FileDesc,
    path: Path,
    last_nread: int
}

struct FileDesc {
    fd: libc::c_int,
    close_on_drop: bool
}

trait AsFileDesc {
    fn as_fd(&self) -> FileDesc;
}

impl AsFileDesc for File {
    fn as_fd(&self) -> FileDesc {
        let hack: HackyFile = unsafe { std::mem::transmute_copy(self) };
        hack.fd
    }
}
// END CODE TO DELETE

#[cfg(target_os = "macos")]
fn rewind_stdout(s: &mut FileDesc) {
    match s.seek(0, io::SeekEnd) {
        Ok(_) => {}
        Err(f) => crash!(1, "{}", f.detail.unwrap())
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int { std::ptr::null() }

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn rewind_stdout(_: &mut FileDesc) {}

pub fn uumain(args: Vec<String>) -> int {
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

    unsafe {
        // we ignore the memory leak here because it doesn't matter anymore
        let executable = opts.free[0].as_slice().to_c_str().unwrap();
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
        let new_stdin = match File::open_mode(&Path::new("/dev/null"), Open, Read) {
            Ok(t) => t,
            Err(e) => {
                crash!(2, "Cannot replace STDIN: {}", e)
            }
        };
        if unsafe { dup2(new_stdin.as_fd().fd, 0) } != 0 {
            crash!(2, "Cannot replace STDIN: {}", std::io::IoError::last_error())
        }
    }

    if replace_stdout {
        let new_stdout = find_stdout();
        let mut fd = new_stdout.as_fd();

        rewind_stdout(&mut fd);

        if unsafe { dup2(fd.fd, 1) } != 1 {
            crash!(2, "Cannot replace STDOUT: {}", std::io::IoError::last_error())
        }
    }

    if replace_stderr {
        if unsafe { dup2(1, 2) } != 2 {
            crash!(2, "Cannot replace STDERR: {}", std::io::IoError::last_error())
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
