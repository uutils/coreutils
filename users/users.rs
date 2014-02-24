#[crate_id(name="users", vers="1.0.0", author="KokaKiwi")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) KokaKiwi <kokakiwi@kokakiwi.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.22 */

// Allow dead code here in order to keep all fields, constants here, for consistency.
#[allow(dead_code, non_camel_case_types)];

#[feature(macro_rules, globs)];

extern crate extra;
extern crate getopts;

use std::io::print;
use std::cast;
use std::libc;
use std::os;
use std::ptr;
use std::str;
use utmpx::*;

#[path = "../util.rs"]
mod util;

#[cfg(target_os = "linux")]
mod utmpx {
    use std::libc;

    pub static DEFAULT_FILE: &'static str = "/var/run/utmp";

    pub static UT_LINESIZE: uint = 32;
    pub static UT_NAMESIZE: uint = 32;
    pub static UT_IDSIZE: uint = 4;
    pub static UT_HOSTSIZE: uint = 256;

    pub static EMPTY: libc::c_short = 0;
    pub static RUN_LVL: libc::c_short = 1;
    pub static BOOT_TIME: libc::c_short = 2;
    pub static NEW_TIME: libc::c_short = 3;
    pub static OLD_TIME: libc::c_short = 4;
    pub static INIT_PROCESS: libc::c_short = 5;
    pub static LOGIN_PROCESS: libc::c_short = 6;
    pub static USER_PROCESS: libc::c_short = 7;
    pub static DEAD_PROCESS: libc::c_short = 8;
    pub static ACCOUNTING: libc::c_short = 9;

    pub struct c_exit_status {
        e_termination: libc::c_short,
        e_exit: libc::c_short,
    }

    pub struct c_utmp {
        ut_type: libc::c_short,
        ut_pid: libc::pid_t,
        ut_line: [libc::c_char, ..UT_LINESIZE],
        ut_id: [libc::c_char, ..UT_IDSIZE],

        ut_user: [libc::c_char, ..UT_NAMESIZE],
        ut_host: [libc::c_char, ..UT_HOSTSIZE],
        ut_exit: c_exit_status,
        ut_session: libc::c_long,
        ut_tv: libc::timeval,

        ut_addr_v6: [libc::int32_t, ..4],
        __unused: [libc::c_char, ..20],
    }
}

#[cfg(target_os = "macos")]
mod utmpx {
    use std::libc;

    pub static DEFAULT_FILE: &'static str = "/var/run/utmpx";

    pub static UT_LINESIZE: uint = 32;
    pub static UT_NAMESIZE: uint = 256;
    pub static UT_IDSIZE: uint = 4;
    pub static UT_HOSTSIZE: uint = 256;

    pub static EMPTY: libc::c_short = 0;
    pub static RUN_LVL: libc::c_short = 1;
    pub static BOOT_TIME: libc::c_short = 2;
    pub static OLD_TIME: libc::c_short = 3;
    pub static NEW_TIME: libc::c_short = 4;
    pub static INIT_PROCESS: libc::c_short = 5;
    pub static LOGIN_PROCESS: libc::c_short = 6;
    pub static USER_PROCESS: libc::c_short = 7;
    pub static DEAD_PROCESS: libc::c_short = 8;
    pub static ACCOUNTING: libc::c_short = 9;

    pub struct c_exit_status {
        e_termination: libc::c_short,
        e_exit: libc::c_short,
    }

    pub struct c_utmp {
        ut_user: [libc::c_char, ..UT_NAMESIZE],
        ut_id: [libc::c_char, ..UT_IDSIZE],
        ut_line: [libc::c_char, ..UT_LINESIZE],
        ut_pid: libc::pid_t,
        ut_type: libc::c_short,
        ut_tv: libc::timeval,
        ut_host: [libc::c_char, ..UT_HOSTSIZE],
        __unused: [libc::c_char, ..16]
    }
}

extern {
    fn getutxent() -> *c_utmp;
    fn getutxid(ut: *c_utmp) -> *c_utmp;
    fn getutxline(ut: *c_utmp) -> *c_utmp;

    fn pututxline(ut: *c_utmp) -> *c_utmp;

    fn setutxent();
    fn endutxent();

    fn utmpxname(file: *libc::c_char) -> libc::c_int;
}

static NAME: &'static str = "users";

fn main() {
    let args = os::args();
    let program = args[0].as_slice();
    let opts = ~[
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_err_msg()),
    };

    if matches.opt_present("help") {
        println!("users 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {:s} [OPTION]... [FILE]", program);
        println!("");
        print(getopts::usage("Output who is currently logged in according to FILE.", opts));
        return;
    }

    if matches.opt_present("version") {
        println!("users 1.0.0");
        return;
    }

    let mut filename = DEFAULT_FILE;
    if matches.free.len() > 0 {
        filename = matches.free[0].as_slice();
    }

    exec(filename);
}

fn exec(filename: &str) {
    filename.with_c_str(|filename| {
        unsafe {
            utmpxname(filename);
        }
    });

    let mut users: ~[~str] = ~[];

    unsafe {
        setutxent();

        loop {
            let line = getutxent();

            if line == ptr::null() {
                break;
            }

            if (*line).ut_type == USER_PROCESS {
                let user = str::raw::from_c_str(cast::transmute(&(*line).ut_user));
                users.push(user);
            }
        }

        endutxent();
    }

    if users.len() > 0 {
        users.sort();
        println!("{}", users.connect(" "));
    }
}
