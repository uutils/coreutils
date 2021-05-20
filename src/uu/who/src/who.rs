// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname hostnames runlevel mesg wtmp statted boottime deadprocs initspawn clockchange curr runlvline pidstr exitstr hoststr

#[macro_use]
extern crate uucore;
use uucore::libc::{ttyname, STDIN_FILENO, S_IWGRP};
use uucore::utmpx::{self, time, Utmpx};

use clap::{App, Arg};
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use uucore::InvalidEncodingHandling;

mod options {
    pub const ALL: &str = "all";
    pub const BOOT: &str = "boot";
    pub const DEAD: &str = "dead";
    pub const HEADING: &str = "heading";
    pub const LOGIN: &str = "login";
    pub const LOOKUP: &str = "lookup";
    pub const ONLY_HOSTNAME_USER: &str = "only_hostname_user";
    pub const PROCESS: &str = "process";
    pub const COUNT: &str = "count";
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    pub const RUNLEVEL: &str = "runlevel";
    pub const SHORT: &str = "short";
    pub const TIME: &str = "time";
    pub const USERS: &str = "users";
    pub const MESG: &str = "mesg"; // aliases: --message, --writable
    pub const FILE: &str = "FILE"; // if length=1: FILE, if length=2: ARG1 ARG2
}

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Print information about users who are currently logged in.";

fn get_usage() -> String {
    format!("{0} [OPTION]... [ FILE | ARG1 ARG2 ]", executable!())
}

fn get_long_usage() -> String {
    format!(
        "If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.\n\
         If ARG1 ARG2 given, -m presumed: 'am i' or 'mom likes' are usual.",
        utmpx::DEFAULT_FILE,
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&after_help[..])
        .arg(
            Arg::with_name(options::ALL)
                .long(options::ALL)
                .short("a")
                .help("same as -b -d --login -p -r -t -T -u"),
        )
        .arg(
            Arg::with_name(options::BOOT)
                .long(options::BOOT)
                .short("b")
                .help("time of last system boot"),
        )
        .arg(
            Arg::with_name(options::DEAD)
                .long(options::DEAD)
                .short("d")
                .help("print dead processes"),
        )
        .arg(
            Arg::with_name(options::HEADING)
                .long(options::HEADING)
                .short("H")
                .help("print line of column headings"),
        )
        .arg(
            Arg::with_name(options::LOGIN)
                .long(options::LOGIN)
                .short("l")
                .help("print system login processes"),
        )
        .arg(
            Arg::with_name(options::LOOKUP)
                .long(options::LOOKUP)
                .help("attempt to canonicalize hostnames via DNS"),
        )
        .arg(
            Arg::with_name(options::ONLY_HOSTNAME_USER)
                .short("m")
                .help("only hostname and user associated with stdin"),
        )
        .arg(
            Arg::with_name(options::PROCESS)
                .long(options::PROCESS)
                .short("p")
                .help("print active processes spawned by init"),
        )
        .arg(
            Arg::with_name(options::COUNT)
                .long(options::COUNT)
                .short("q")
                .help("all login names and number of users logged on"),
        )
        .arg(
            #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
            Arg::with_name(options::RUNLEVEL)
                .long(options::RUNLEVEL)
                .short("r")
                .help("print current runlevel"),
        )
        .arg(
            Arg::with_name(options::SHORT)
                .long(options::SHORT)
                .short("s")
                .help("print only name, line, and time (default)"),
        )
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .short("t")
                .help("print last system clock change"),
        )
        .arg(
            Arg::with_name(options::USERS)
                .long(options::USERS)
                .short("u")
                .help("list users logged in"),
        )
        .arg(
            Arg::with_name(options::MESG)
                .long(options::MESG)
                .short("T")
                // .visible_short_alias('w')  // TODO: requires clap "3.0.0-beta.2"
                .visible_aliases(&["message", "writable"])
                .help("add user's message status as +, - or ?"),
        )
        .arg(
            Arg::with_name("w") // work around for `Arg::visible_short_alias`
                .short("w")
                .help("same as -T"),
        )
        .arg(
            Arg::with_name(options::FILE)
                .takes_value(true)
                .min_values(1)
                .max_values(2),
        )
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(options::FILE)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    // If true, attempt to canonicalize hostnames via a DNS lookup.
    let do_lookup = matches.is_present(options::LOOKUP);

    // If true, display only a list of usernames and count of
    // the users logged on.
    // Ignored for 'who am i'.
    let short_list = matches.is_present(options::COUNT);

    // If true, display only name, line, and time fields.
    let mut short_output = false;

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or "." if within the last minute, or "old" if
    // not within the last day.
    let mut include_idle = false;

    // If true, display a line at the top describing each field.
    let include_heading = matches.is_present(options::HEADING);

    // If true, display a '+' for each user if mesg y, a '-' if mesg n,
    // or a '?' if their tty cannot be statted.
    let include_mesg = matches.is_present(options::ALL)
        || matches.is_present(options::MESG)
        || matches.is_present("w");

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

    #[allow(clippy::useless_let_if_seq)]
    {
        if matches.is_present(options::ALL) {
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

        if matches.is_present(options::BOOT) {
            need_boottime = true;
            assumptions = false;
        }

        if matches.is_present(options::DEAD) {
            need_deadprocs = true;
            include_idle = true;
            include_exit = true;
            assumptions = false;
        }

        if matches.is_present(options::LOGIN) {
            need_login = true;
            include_idle = true;
            assumptions = false;
        }

        if matches.is_present(options::ONLY_HOSTNAME_USER) || files.len() == 2 {
            my_line_only = true;
        }

        if matches.is_present(options::PROCESS) {
            need_initspawn = true;
            assumptions = false;
        }

        if matches.is_present(options::RUNLEVEL) {
            need_runlevel = true;
            include_idle = true;
            assumptions = false;
        }

        if matches.is_present(options::SHORT) {
            short_output = true;
        }

        if matches.is_present(options::TIME) {
            need_clockchange = true;
            assumptions = false;
        }

        if matches.is_present(options::USERS) {
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
    }

    let mut who = Who {
        do_lookup,
        short_list,
        short_output,
        include_idle,
        include_heading,
        include_mesg,
        include_exit,
        need_boottime,
        need_deadprocs,
        need_login,
        need_initspawn,
        need_clockchange,
        need_runlevel,
        need_users,
        my_line_only,
        args: files,
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
                format!(
                    "{:02}:{:02}",
                    seconds_idle / 3600,
                    (seconds_idle % 3600) / 60
                )
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
            CStr::from_ptr(res as *const _)
                .to_string_lossy()
                .trim_start_matches("/dev/")
                .to_owned()
        } else {
            "".to_owned()
        }
    }
}

impl Who {
    fn exec(&mut self) {
        let run_level_chk = |record: i16| {
            #[allow(unused_assignments)]
            let mut res = false;

            #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
            {
                res = record == utmpx::RUN_LVL;
            }
            res
        };

        let f = if self.args.len() == 1 {
            self.args[0].as_ref()
        } else {
            utmpx::DEFAULT_FILE
        };
        if self.short_list {
            let users = Utmpx::iter_all_records()
                .read_from(f)
                .filter(Utmpx::is_user_process)
                .map(|ut| ut.user())
                .collect::<Vec<_>>();
            println!("{}", users.join(" "));
            println!("# users={}", users.len());
        } else {
            let records = Utmpx::iter_all_records().read_from(f).peekable();

            if self.include_heading {
                self.print_heading()
            }
            let cur_tty = if self.my_line_only {
                current_tty()
            } else {
                "".to_owned()
            };

            for ut in records {
                if !self.my_line_only || cur_tty == ut.tty_device() {
                    if self.need_users && ut.is_user_process() {
                        self.print_user(&ut);
                    } else if self.need_runlevel && run_level_chk(ut.record_type()) {
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

                if ut.record_type() == utmpx::BOOT_TIME {}
            }
        }
    }

    #[inline]
    fn print_runlevel(&self, ut: &Utmpx) {
        let last = (ut.pid() / 256) as u8 as char;
        let curr = (ut.pid() % 256) as u8 as char;
        let runlvline = format!("run-level {}", curr);
        let comment = format!("last={}", if last == 'N' { 'S' } else { 'N' });

        self.print_line(
            "",
            ' ',
            &runlvline,
            &time_string(ut),
            "",
            "",
            if !last.is_control() { &comment } else { "" },
            "",
        );
    }

    #[inline]
    fn print_clockchange(&self, ut: &Utmpx) {
        self.print_line("", ' ', "clock change", &time_string(ut), "", "", "", "");
    }

    #[inline]
    fn print_login(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        self.print_line(
            "LOGIN",
            ' ',
            &ut.tty_device(),
            &time_string(ut),
            "",
            &pidstr,
            &comment,
            "",
        );
    }

    #[inline]
    fn print_deadprocs(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        let e = ut.exit_status();
        let exitstr = format!("term={} exit={}", e.0, e.1);
        self.print_line(
            "",
            ' ',
            &ut.tty_device(),
            &time_string(ut),
            "",
            &pidstr,
            &comment,
            &exitstr,
        );
    }

    #[inline]
    fn print_initspawn(&self, ut: &Utmpx) {
        let comment = format!("id={}", ut.terminal_suffix());
        let pidstr = format!("{}", ut.pid());
        self.print_line(
            "",
            ' ',
            &ut.tty_device(),
            &time_string(ut),
            "",
            &pidstr,
            &comment,
            "",
        );
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

        let mut s = ut.host();
        if self.do_lookup {
            s = safe_unwrap!(ut.canon_host());
        }
        let hoststr = if s.is_empty() { s } else { format!("({})", s) };

        self.print_line(
            ut.user().as_ref(),
            mesg,
            ut.tty_device().as_ref(),
            time_string(ut).as_str(),
            idle.as_ref(),
            format!("{}", ut.pid()).as_str(),
            hoststr.as_str(),
            "",
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn print_line(
        &self,
        user: &str,
        state: char,
        line: &str,
        time: &str,
        idle: &str,
        pid: &str,
        comment: &str,
        exit: &str,
    ) {
        let mut buf = String::with_capacity(64);
        let msg = vec![' ', state].into_iter().collect::<String>();

        buf.push_str(&format!("{:<8}", user));
        if self.include_mesg {
            buf.push_str(&msg);
        }
        buf.push_str(&format!(" {:<12}", line));
        // "%Y-%m-%d %H:%M"
        let time_size = 4 + 1 + 2 + 1 + 2 + 1 + 2 + 1 + 2;
        buf.push_str(&format!(" {:<1$}", time, time_size));

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
        println!("{}", buf.trim_end());
    }

    #[inline]
    fn print_heading(&self) {
        self.print_line(
            "NAME", ' ', "LINE", "TIME", "IDLE", "PID", "COMMENT", "EXIT",
        );
    }
}
