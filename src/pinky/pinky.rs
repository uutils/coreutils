#![crate_name = "uu_pinky"]

// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate uucore;
use uucore::c_types::getpwnam;

extern crate getopts;
extern crate libc;
use libc::{uid_t, gid_t, c_char};

use std::io::prelude::*;
use std::io::BufReader;
use std::ptr;
use std::fs::File;
use std::ffi::CStr;
use std::path::PathBuf;

static NAME: &'static str = "pinky";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

const BUFSIZE: usize = 1024;

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("l",
                 "l",
                 "produce long format output for the specified USERs");
    opts.optflag("b",
                 "b",
                 "omit the user's home directory and shell in long format");
    opts.optflag("h", "h", "omit the user's project file in long format");
    opts.optflag("p", "p", "omit the user's plan file in long format");
    opts.optflag("s", "s", "do short format output, this is the default");
    opts.optflag("f", "f", "omit the line of column headings in short format");
    opts.optflag("w", "w", "omit the user's full name in short format");
    opts.optflag("i",
                 "i",
                 "omit the user's full name and remote host in short format");
    opts.optflag("q",
                 "q",
                 "omit the user's full name, remote host and idle time in short format");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            disp_err!("{}", f);
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("Usage: {} [OPTION]... [USER]...

  -l              produce long format output for the specified USERs
  -b              omit the user's home directory and shell in long format
  -h              omit the user's project file in long format
  -p              omit the user's plan file in long format
  -s              do short format output, this is the default
  -f              omit the line of column headings in short format
  -w              omit the user's full name in short format
  -i              omit the user's full name and remote host in short format
  -q              omit the user's full name, remote host and idle time
                  in short format
      --help     display this help and exit
      --version  output version information and exit

A lightweight 'finger' program;  print user information.
The utmp file will be {}",
                 NAME,
                 "/var/run/utmp");
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or blank if within the last minute, or days followed
    // by a 'd' if not within the last day.
    let mut include_idle = true;

    // If true, display a line at the top describing each field.
    let include_heading = !matches.opt_present("f");

    // if true, display the user's full name from pw_gecos.
    let mut include_fullname = true;

    // if true, display the user's ~/.project file when doing long format.
    let include_project = !matches.opt_present("h");

    // if true, display the user's ~/.plan file when doing long format.
    let include_plan = !matches.opt_present("p");

    // if true, display the user's home directory and shell
    // when doing long format.
    let include_home_and_shell = !matches.opt_present("b");

    // if true, use the "short" output format.
    let do_short_format = !matches.opt_present("l");

    if matches.opt_present("w") {
        include_fullname = false;
    }
    if matches.opt_present("i") {
        include_fullname = false;
        // FIXME:
    }
    if matches.opt_present("q") {
        include_fullname = false;
        include_idle = false;
        // FIXME:
    }

    if !do_short_format && matches.free.is_empty() {
        disp_err!("no username specified; at least one must be specified when using -l");
        return 1;
    }

    let pk = Pinky {
        include_idle: include_idle,
        include_heading: include_heading,
        include_fullname: include_fullname,
        include_project: include_project,
        include_plan: include_plan,
        include_home_and_shell: include_home_and_shell,
        do_short_format: do_short_format,
        users: matches.free,
    };

    if do_short_format {
        pk.short_pinky()
    } else {
        pk.long_pinky()
    }

}

struct Pinky {
    include_idle: bool,
    include_heading: bool,
    include_fullname: bool,
    include_project: bool,
    include_plan: bool,
    include_home_and_shell: bool,
    do_short_format: bool,
    users: Vec<String>,
}

#[derive(Debug)]
struct Passwd {
    pw_name: String,
    pw_passwd: String,
    pw_uid: uid_t,
    pw_gid: gid_t,
    pw_gecos: String,
    pw_dir: String,
    pw_shell: String,
}

fn getpw(u: &str) -> Option<Passwd> {
    let pw = unsafe { getpwnam(u.as_ptr() as *const i8) };
    if !pw.is_null() {
        let data = unsafe { ptr::read(pw) };
        Some(Passwd {
            pw_name: cstr2string(data.pw_name),
            pw_passwd: cstr2string(data.pw_passwd),
            pw_uid: data.pw_uid,
            pw_gid: data.pw_gid,
            pw_dir: cstr2string(data.pw_dir),
            pw_gecos: cstr2string(data.pw_gecos),
            pw_shell: cstr2string(data.pw_shell),
        })
    } else {
        None
    }
}

#[inline]
fn cstr2string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        println!("null ptr");
        return "".to_owned();
    }
    let s = unsafe { CStr::from_ptr(ptr) };
    s.to_string_lossy().into_owned()
}


trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for str {
    fn capitalize(&self) -> String {
        use std::ascii::AsciiExt;
        self.char_indices().fold(String::with_capacity(self.len()), |mut acc, x| {
            if x.0 != 0 {
                acc.push(x.1)
            } else {
                acc.push(x.1.to_ascii_uppercase())
            }
            acc
        })
    }
}

impl Pinky {
    fn short_pinky(&self) -> i32 {
        0
    }

    fn long_pinky(&self) -> i32 {
        for u in &self.users {
            print!("Login name: {:<28}In real life: ", u);
            if let Some(pw) = getpw(u) {
                println!(" {}", pw.pw_gecos.replace("&", &pw.pw_name.capitalize()));
                if self.include_home_and_shell {
                    print!("Directory: {:<29}", pw.pw_dir);
                    println!("Shell:  {}", pw.pw_shell);
                }
                if self.include_project {
                    let mut p = PathBuf::from(&pw.pw_dir);
                    p.push(".project");
                    if let Ok(f) = File::open(p) {
                        print!("Project: ");
                        read_to_console(f);
                    }
                }
                if self.include_plan {
                    let mut p = PathBuf::from(&pw.pw_dir);
                    p.push(".plan");
                    if let Ok(f) = File::open(p) {
                        println!("Plan:");
                        read_to_console(f);
                    }
                }
                println!("");
            } else {
                println!(" ???");
            }
        }
        0
    }
}

fn read_to_console<F: Read>(f: F) {
    let mut reader = BufReader::new(f);
    let mut iobuf = [0_u8; BUFSIZE];
    while let Ok(n) = reader.read(&mut iobuf) {
        if n == 0 {
            break;
        }
        let s = String::from_utf8_lossy(&iobuf);
        print!("{}", s);
    }
}
