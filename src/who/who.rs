// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
#![crate_name = "uu_who"]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate uucore;
use uucore::utmpx::{self, time, Utmpx};
use uucore::libc::{STDIN_FILENO, ttyname, S_IWGRP};

use std::borrow::Cow;
use std::io::prelude::*;
use std::ffi::CStr;
use std::path::PathBuf;
use std::os::unix::fs::MetadataExt;

static SYNTAX: &'static str = "[OPTION]... [ FILE | ARG1 ARG2 ]";
static SUMMARY: &'static str = "Print information about users who are currently logged in.";
static LONG_HELP: &'static str = "
  -a, --all         same as -b -d --login -p -r -t -T -u
  -b, --boot        time of last system boot
  -d, --dead        print dead processes
  -H, --heading     print line of column headings
  -l, --login       print system login processes
      --lookup      attempt to canonicalize hostnames via DNS
  -m                only hostname and user associated with stdin
  -p, --process     print active processes spawned by init
  -q, --count       all login names and number of users logged on
  -r, --runlevel    print current runlevel
  -s, --short       print only name, line, and time (default)
  -t, --time        print last system clock change
  -T, -w, --mesg    add user's message status as +, - or ?
  -u, --users       list users logged in
      --message     same as -T
      --writable    same as -T
      --help     display this help and exit
      --version  output version information and exit

If FILE is not specified, use /var/run/utmp.  /var/log/wtmp as FILE is common.
If ARG1 ARG2 given, -m presumed: 'am i' or 'mom likes' are usual.
";

pub fn uumain(args: Vec<String>) -> i32 {

    let mut opts = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP);
    opts.optflag("a", "all", "same as -b -d --login -p -r -t -T -u");
    opts.optflag("b", "boot", "time of last system boot");
    opts.optflag("d", "dead", "print dead processes");
    opts.optflag("H", "heading", "print line of column headings");
    opts.optflag("l", "login", "print system login processes");
    opts.optflag("", "lookup", "attempt to canonicalize hostnames via DNS");
    opts.optflag("m", "m", "only hostname and user associated with stdin");
    opts.optflag("p", "process", "print active processes spawned by init");
    opts.optflag("q",
                 "count",
                 "all login names and number of users logged on");
    opts.optflag("r", "runlevel", "print current runlevel");
    opts.optflag("s", "short", "print only name, line, and time (default)");
    opts.optflag("t", "time", "print last system clock change");
    opts.optflag("u", "users", "list users logged in");
    opts.optflag("w", "mesg", "add user's message status as +, - or ?");
    // --message, --writable are the same as --mesg
    opts.optflag("T", "message", "");
    opts.optflag("T", "writable", "");

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = opts.parse(args);

    // If true, attempt to canonicalize hostnames via a DNS lookup.
    let do_lookup = matches.opt_present("lookup");

    // If true, display only a list of usernames and count of
    // the users logged on.
    // Ignored for 'who am i'.
    let short_list = matches.opt_present("q");

    // If true, display only name, line, and time fields.
    let mut short_output = false;

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or "." if within the last minute, or "old" if
    // not within the last day.
    let mut include_idle = false;

    // If true, display a line at the top describing each field.
    let include_heading = matches.opt_present("H");

    // If true, display a '+' for each user if mesg y, a '-' if mesg n,
    // or a '?' if their tty cannot be statted.
    let include_mesg = matches.opt_present("a") || matches.opt_present("T") || matches.opt_present("w");

    // If true, display process termination & exit status.
    let mut include_exit = false;

    // If true, display the last boot time.
    let mut need_boottime = false;

    // If true, display dead processes.
    let mut need_deadprocs = false;

    // If true, display processes waiting for user login.
    let mut need_login = false;

    // If true, display processes started by init.
    let mut need_initspawn = false;

    // If true, display the last clock change.
    let mut need_clockchange = false;

    // If true, display the current runlevel.
    let mut need_runlevel = false;

    // If true, display user processes.
    let mut need_users = false;

    // If true, display info only for the controlling tty.
    let mut my_line_only = false;

    let mut assumptions = true;

    if matches.opt_present("a") {
        need_boottime = true;
        need_deadprocs = true;
        need_login = true;
        need_initspawn = true;
        need_runlevel = true;
        need_clockchange = true;
        need_users = true;
        include_idle = true;
        include_exit = true;
        assumptions = false;
    }

    if matches.opt_present("b") {
        need_boottime = true;
        assumptions = false;
    }

    if matches.opt_present("d") {
        need_deadprocs = true;
        include_idle = true;
        include_exit = true;
        assumptions = false;
    }

    if matches.opt_present("l") {
        need_login = true;
        include_idle = true;
        assumptions = false;
    }

    if matches.opt_present("m") || matches.free.len() == 2 {
        my_line_only = true;
    }

    if matches.opt_present("p") {
        need_initspawn = true;
        assumptions = false;
    }

    if matches.opt_present("r") {
        need_runlevel = true;
        include_idle = true;
        assumptions = false;
    }

    if matches.opt_present("s") {
        short_output = true;
    }

    if matches.opt_present("t") {
        need_clockchange = true;
        assumptions = false;
    }

    if matches.opt_present("u") {
        need_users = true;
        include_idle = true;
        assumptions = false;
    }

    if assumptions {
        need_users = true;
        short_output = true;
    }

    if include_exit {
        short_output = false;
    }

    if matches.free.len() > 2 {
        disp_err!("{}", msg_wrong_number_of_arguments!());
        exit!(1);
    }

    let who = Who {
        do_lookup: do_lookup,
        short_list: short_list,
        short_output: short_output,
        include_idle: include_idle,
        include_heading: include_heading,
        include_mesg: include_mesg,
        include_exit: include_exit,
        need_boottime: need_boottime,
        need_deadprocs: need_deadprocs,
        need_login: need_login,
        need_initspawn: need_initspawn,
        need_clockchange: need_clockchange,
        need_runlevel: need_runlevel,
        need_users: need_users,
        my_line_only: my_line_only,
        args: matches.free,
    };

    who.exec();

    0
}

struct Who {
    do_lookup: bool,
    short_list: bool,
    short_output: bool,
    include_idle: bool,
    include_heading: bool,
    include_mesg: bool,
    include_exit: bool,
    need_boottime: bool,
    need_deadprocs: bool,
    need_login: bool,
    need_initspawn: bool,
    need_clockchange: bool,
    need_runlevel: bool,
    need_users: bool,
    my_line_only: bool,
    args: Vec<String>,
}

fn idle_string<'a>(when: i64, boottime: i64) -> Cow<'a, str> {
    thread_local! {
        static NOW: time::Tm = time::now()
    }
    NOW.with(|n| {
        let now = n.to_timespec().sec;
        if boottime < when && now - 24 * 3600 < when && when <= now {
            let seconds_idle = now - when;
            if seconds_idle < 60 {
                "  .  ".into()
            } else {
                format!("{:02}:{:02}",
                        seconds_idle / 3600,
                        (seconds_idle % 3600) / 60)
                        .into()
            }
        } else {
            " old ".into()
        }
    })
}

fn time_string(ut: &Utmpx) -> String {
    time::strftime("%Y-%m-%d %H:%M", &ut.login_time()).unwrap()
}

#[inline]
fn current_tty() -> String {
    unsafe {
        let res = ttyname(STDIN_FILENO);
        if !res.is_null() {
            CStr::from_ptr(res as *const _).to_string_lossy().trim_left_matches("/dev/").to_owned()
        } else {
            "".to_owned()
        }
    }
}

impl Who {
    fn exec(&self) {
        let f = if self.args.len() == 1 {
            self.args[0].as_ref()
        } else {
            utmpx::DEFAULT_FILE
        };
        if self.short_list {
            let users = Utmpx::iter_all_records()
                .read_from(f)
                .filter(|ut| ut.is_user_process())
                .map(|ut| ut.user())
                .collect::<Vec<_>>();
            println!("{}", users.join(" "));
            println!("# users={}", users.len());
        } else {
            if self.include_heading {
                self.print_heading()
            }
            let cur_tty = if self.my_line_only {
                current_tty()
            } else {
                "".to_owned()
            };

            for ut in Utmpx::iter_all_records().read_from(f) {
                if !self.my_line_only || cur_tty == ut.tty_device() {
                    if self.need_users && ut.is_user_process() {
                        self.print_user(&ut);
                    } else if self.need_runlevel && ut.record_type() == utmpx::RUN_LVL {
                        self.print_runlevel(&ut);
                    } else if self.need_boottime && ut.record_type() == utmpx::BOOT_TIME {
                        self.print_boottime(&ut);
                    } else if self.need_clockchange && ut.record_type() == utmpx::NEW_TIME {
                        self.print_clockchange(&ut);
                    } else if self.need_initspawn && ut.record_type() == utmpx::INIT_PROCESS {
                        self.print_initspawn(&ut);
                    } else if self.need_login && ut.record_type() == utmpx::LOGIN_PROCESS {
                        self.print_login(&ut);
                    } else if self.need_deadprocs && ut.record_type() == utmpx::DEAD_PROCESS {
                        self.print_deadprocs(&ut);
                    }
                }

                if ut.record_type() == utmpx::BOOT_TIME {

                }
            }
        }
    }

    #[inline]
    fn print_runlevel(&self, ut: &Utmpx) {
        let last = (ut.pid() / 256) as u8 as char;
        let curr = (ut.pid() % 256) as u8 as char;
        let runlvline = format!("run-level {}", curr);
        let comment = format!("last={}",
                              if last == 'N' {
                                  'S'
                              } else {
                                  'N'
                              });

        self.print_line("",
                        ' ',
                        &runlvline,
                        &time_string(ut),
                        "",
                        "",
                        if !last.is_control() {
                            &comment
                        } else {
                            ""
                        },
                        "");
    }

    #[inline]
    fn print_clockchange(&self, ut: &Utmpx) {
        self.print_line("", ' ', "clock change", &time_string(ut), "", "", "", "");
    }

    #[inline]
    fn print_login(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        self.print_line("LOGIN",
                        ' ',
                        &ut.tty_device(),
                        &time_string(ut),
                        "",
                        &pidstr,
                        &comment,
                        "");
    }

    #[inline]
    fn print_deadprocs(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        let e = ut.exit_status();
        let exitstr = format!("term={} exit={}", e.0, e.1);
        self.print_line("",
                        ' ',
                        &ut.tty_device(),
                        &time_string(ut),
                        "",
                        &pidstr,
                        &comment,
                        &exitstr);
    }

    #[inline]
    fn print_initspawn(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        self.print_line("",
                        ' ',
                        &ut.tty_device(),
                        &time_string(ut),
                        "",
                        &pidstr,
                        &comment,
                        "");
    }

    #[inline]
    fn print_boottime(&self, ut: &Utmpx) {
        self.print_line("", ' ', "system boot", &time_string(ut), "", "", "", "");
    }

    fn print_user(&self, ut: &Utmpx) {
        let mut p = PathBuf::from("/dev");
        p.push(ut.tty_device().as_str());
        let mesg;
        let last_change;
        match p.metadata() {
            Ok(meta) => {
                mesg = if meta.mode() & (S_IWGRP as u32) != 0 {
                    '+'
                } else {
                    '-'
                };
                last_change = meta.atime();
            }
            _ => {
                mesg = '?';
                last_change = 0;
            }
        }

        let idle = if last_change != 0 {
            idle_string(last_change, 0)
        } else {
            "  ?".into()
        };

        let mut buf = vec![];
        let ut_host = ut.host();
        let mut res = ut_host.splitn(2, ':');
        if let Some(h) = res.next() {
            if self.do_lookup {
                buf.push(ut.canon_host().unwrap_or(h.to_owned()));
            } else {
                buf.push(h.to_owned());
            }
        }
        if let Some(h) = res.next() {
            buf.push(h.to_owned());
        }
        let s = buf.join(":");
        let hoststr = if s.is_empty() {
            s
        } else {
            format!("({})", s)
        };

        self.print_line(ut.user().as_ref(),
                        mesg,
                        ut.tty_device().as_ref(),
                        time_string(ut).as_str(),
                        idle.as_ref(),
                        format!("{}", ut.pid()).as_str(),
                        hoststr.as_str(),
                        "");
    }

    fn print_line(&self,
                  user: &str,
                  state: char,
                  line: &str,
                  time: &str,
                  idle: &str,
                  pid: &str,
                  comment: &str,
                  exit: &str) {
        let mut buf = String::with_capacity(64);
        let msg = vec![' ', state].into_iter().collect::<String>();

        buf.push_str(&format!("{:<8}", user));
        if self.include_mesg {
            buf.push_str(&msg);
        }
        buf.push_str(&format!(" {:<12}", line));
        // "%Y-%m-%d %H:%M"
        buf.push_str(&format!(" {:<1$}", time, 4 + 1 + 2 + 1 + 2 + 1 + 2 + 1 + 2));

        if !self.short_output {
            if self.include_idle {
                buf.push_str(&format!(" {:<6}", idle));
            }
            buf.push_str(&format!(" {:>10}", pid));
        }
        buf.push_str(&format!(" {:<8}", comment));
        if self.include_exit {
            buf.push_str(&format!(" {:<12}", exit));
        }
        println!("{}", buf.trim_right());
    }

    #[inline]
    fn print_heading(&self) {
        self.print_line("NAME",
                        ' ',
                        "LINE",
                        "TIME",
                        "IDLE",
                        "PID",
                        "COMMENT",
                        "EXIT");
    }
}
