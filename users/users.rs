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
#[allow(dead_code)];

extern mod extra;

use std::io::print;
use std::cast;
use std::libc;
use std::os;
use std::ptr;
use std::str;
use extra::getopts::groups;

static DEFAULT_FILE: &'static str = "/var/run/utmp";

static UT_LINESIZE: uint = 32;
static UT_NAMESIZE: uint = 32;
static UT_HOSTSIZE: uint = 256;

static EMPTY: libc::c_short = 0;
static RUN_LVL: libc::c_short = 1;
static BOOT_TIME: libc::c_short = 2;
static NEW_TIME: libc::c_short = 3;
static OLD_TIME: libc::c_short = 4;
static INIT_PROCESS: libc::c_short = 5;
static LOGIN_PROCESS: libc::c_short = 6;
static USER_PROCESS: libc::c_short = 7;
static DEAD_PROCESS: libc::c_short = 8;
static ACCOUNTING: libc::c_short = 9;

struct c_exit_status {
    e_termination: libc::c_short,
    e_exit: libc::c_short,
}

struct c_utmp {
    ut_type: libc::c_short,
    ut_pid: libc::pid_t,
    ut_line: [libc::c_char, ..UT_LINESIZE],
    ut_id: [libc::c_char, ..4],

    ut_user: [libc::c_char, ..UT_NAMESIZE],
    ut_host: [libc::c_char, ..UT_HOSTSIZE],
    ut_exit: c_exit_status,
    ut_session: libc::c_long,
    ut_tv: libc::timeval,

    ut_addr_v6: [libc::int32_t, ..4],
    __unused: [libc::c_char, ..20],
}

extern {
    fn getutent() -> *c_utmp;
    fn getutid(ut: *c_utmp) -> *c_utmp;
    fn getutline(ut: *c_utmp) -> *c_utmp;

    fn pututline(ut: *c_utmp) -> *c_utmp;

    fn setutent();
    fn endutent();

    fn utmpname(file: *libc::c_char) -> libc::c_int;
}

fn main() {
    let args = os::args();
    let program = args[0].as_slice();
    let opts = ~[
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_err_msg()),
    };

    if matches.opt_present("help") {
        println!("users 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {:s} [OPTION]... [FILE]", program);
        println!("");
        print(groups::usage("Output who is currently logged in according to FILE.", opts));
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
            utmpname(filename);
        }
    });

    let mut users: ~[~str] = ~[];

    unsafe {
        setutent();

        loop {
            let line = getutent();

            if line == ptr::null() {
                break;
            }

            if (*line).ut_type == USER_PROCESS {
                let user = str::raw::from_c_str(cast::transmute(&(*line).ut_user));
                users.push(user);
            }
        }

        endutent();
    }

    if users.len() > 0 {
        users.sort();
        println!("{}", users.connect(" "));
    }
}
