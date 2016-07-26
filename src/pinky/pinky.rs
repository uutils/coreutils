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
use uucore::utmpx;

extern crate getopts;
extern crate libc;
use libc::{uid_t, gid_t, c_char};
use libc::S_IWGRP;

extern crate time;

use std::io::prelude::*;
use std::io::BufReader;
use std::io::Result as IOResult;

use std::fs::File;
use std::os::unix::fs::MetadataExt;

use std::ptr;
use std::ffi::{CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;

use std::path::Path;
use std::path::PathBuf;

mod utmp;

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
                 utmpx::DEFAULT_FILE);
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

    /* if true, display the ut_host field. */
    let mut include_where = true;

    if matches.opt_present("w") {
        include_fullname = false;
    }
    if matches.opt_present("i") {
        include_fullname = false;
        include_where = false;
    }
    if matches.opt_present("q") {
        include_fullname = false;
        include_idle = false;
        include_where = false;
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
        include_where: include_where,
        names: matches.free,
    };

    if do_short_format {
        if let Err(e) = pk.short_pinky() {
            disp_err!("{}", e);
            1
        } else {
            0
        }
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
    include_where: bool,
    include_home_and_shell: bool,
    names: Vec<String>,
}

#[derive(Debug)]
pub struct Passwd {
    pw_name: String,
    pw_passwd: String,
    pw_uid: uid_t,
    pw_gid: gid_t,
    pw_gecos: String,
    pw_dir: String,
    pw_shell: String,
}

trait FromChars {
    fn from_chars(*const c_char) -> Self;
}

impl FromChars for String {
    #[inline]
    fn from_chars(ptr: *const c_char) -> Self {
        if ptr.is_null() {
            return "".to_owned();
        }
        let s = unsafe { CStr::from_ptr(ptr) };
        s.to_string_lossy().into_owned()
    }
}

pub fn getpw(u: &str) -> Option<Passwd> {
    let pw = unsafe {
        getpwnam(CString::new(u).unwrap().as_ptr())
    };
    if !pw.is_null() {
        let data = unsafe { ptr::read(pw) };
        Some(Passwd {
            pw_name: String::from_chars(data.pw_name),
            pw_passwd: String::from_chars(data.pw_passwd),
            pw_uid: data.pw_uid,
            pw_gid: data.pw_gid,
            pw_dir: String::from_chars(data.pw_dir),
            pw_gecos: String::from_chars(data.pw_gecos),
            pw_shell: String::from_chars(data.pw_shell),
        })
    } else {
        None
    }
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

trait UtmpUtils {
    fn is_user_process(&self) -> bool;
}

impl UtmpUtils for utmpx::c_utmp {
    fn is_user_process(&self) -> bool {
        self.ut_user[0] != 0 && self.ut_type == utmpx::USER_PROCESS
    }
}

fn idle_string(when: i64) -> String {
    thread_local! {
        static NOW: time::Tm = time::now()
    }
    NOW.with(|n| {
        let duration = n.to_timespec().sec - when;
        if duration < 60 {
            // less than 1min
            "     ".to_owned()
        } else if duration < 24 * 3600 {
            // less than 1day
            let hours = duration / (60 * 60);
            let minutes = (duration % (60 * 60)) / 60;
            format!("{:02}:{:02}", hours, minutes)
        } else {
            // more than 1day
            let days = duration / (24 * 3600);
            format!("{}d", days)
        }
    })
}

fn time_string(ut: &utmpx::c_utmp) -> String {
    let tm = time::at(time::Timespec::new(ut.ut_tv.tv_sec, ut.ut_tv.tv_usec as i32));
    time::strftime("%Y-%m-%d %H:%M", &tm).unwrap()
}

impl Pinky {
    fn print_entry(&self, ut: &utmpx::c_utmp) {
        let mut pts_path = PathBuf::from("/dev");
        let line: &Path = OsStr::from_bytes(unsafe {
            CStr::from_ptr(ut.ut_line.as_ref().as_ptr()).to_bytes()
        }).as_ref();
        pts_path.push(line);

        let mesg;
        let last_change;
        match pts_path.metadata() {
            Ok(meta) => {
                mesg = if meta.mode() & (S_IWGRP as u32) != 0 {
                    ' '
                } else {
                    '*'
                };
                last_change = meta.atime();
            }
            _ => {
                mesg = '?';
                last_change = 0;
            }
        }

        let ut_user = String::from_chars(ut.ut_user.as_ref().as_ptr());
        print!("{1:<8.0$}", utmpx::UT_NAMESIZE, ut_user);

        if self.include_fullname {
            if let Some(pw) = getpw(&ut_user) {
                let mut gecos = pw.pw_gecos;
                if let Some(n) = gecos.find(',') {
                    gecos.truncate(n + 1);
                }
                print!(" {:<19.19}", gecos.replace("&", &pw.pw_name.capitalize()));
            } else {
                print!(" {:19}", "        ???");
            }

        }

        print!(" {}{:<8.*}", mesg, utmpx::UT_LINESIZE, String::from_chars(ut.ut_line.as_ref().as_ptr()));

        if self.include_idle {
            if last_change != 0 {
                print!(" {:<6}", idle_string(last_change));
            } else {
                print!(" {:<6}", "?????");
            }
        }

        print!(" {}", time_string(&ut));

        if self.include_where && ut.ut_host[0] != 0 {
            let ut_host = String::from_chars(ut.ut_host.as_ref().as_ptr());
            //if let Some(n) = ut_host.find(':') {
                //ut_host.truncate(n + 1);
            //}
            print!(" {}", ut_host);
        }

        println!("");
    }

    fn print_heading(&self) {
        print!("{:<8}", "Login");
        if self.include_fullname {
            print!(" {:<19}", "Name");
        }
        print!(" {:<9}", " TTY");
        if self.include_idle {
            print!(" {:<6}", "Idle");
        }
        print!(" {:<16}", "When");
        if self.include_where {
            print!(" Where");
        }
        println!("");
    }

    fn short_pinky(&self) -> IOResult<()> {
        if self.include_heading {
            self.print_heading();
        }
        for ut in utmp::read_utmps() {
            if ut.is_user_process() {
                if self.names.is_empty() {
                    self.print_entry(&ut)
                } else {
                    let ut_user = unsafe {
                        CStr::from_ptr(ut.ut_user.as_ref().as_ptr()).to_bytes()
                    };
                    if self.names.iter().any(|n| n.as_bytes() == ut_user) {
                        self.print_entry(&ut);
                    }
                }
            }
        }
        Ok(())
    }

    fn long_pinky(&self) -> i32 {
        for u in &self.names {
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
