// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname hostnames runlevel mesg wtmp statted boottime deadprocs initspawn clockchange curr runlvline pidstr exitstr hoststr

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::libc::{ttyname, STDIN_FILENO, S_IWGRP};
use uucore::utmpx::{self, time, Utmpx};

use clap::{crate_version, Arg, ArgAction, Command};
use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use uucore::format_usage;

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
    pub const RUNLEVEL: &str = "runlevel";
    pub const SHORT: &str = "short";
    pub const TIME: &str = "time";
    pub const USERS: &str = "users";
    pub const MESG: &str = "mesg"; // aliases: --message, --writable
    pub const FILE: &str = "FILE"; // if length=1: FILE, if length=2: ARG1 ARG2
}

static ABOUT: &str = "Print information about users who are currently logged in.";
const USAGE: &str = "{} [OPTION]... [ FILE | ARG1 ARG2 ]";

#[cfg(target_os = "linux")]
static RUNLEVEL_HELP: &str = "print current runlevel";
#[cfg(not(target_os = "linux"))]
static RUNLEVEL_HELP: &str = "print current runlevel (This is meaningless on non Linux)";

fn get_long_usage() -> String {
    format!(
        "If FILE is not specified, use {}.  /var/log/wtmp as FILE is common.\n\
         If ARG1 ARG2 given, -m presumed: 'am i' or 'mom likes' are usual.",
        utmpx::DEFAULT_FILE,
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app()
        .after_help(get_long_usage())
        .try_get_matches_from(args)?;

    let files: Vec<String> = matches
        .get_many::<String>(options::FILE)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    // If true, attempt to canonicalize hostnames via a DNS lookup.
    let do_lookup = matches.get_flag(options::LOOKUP);

    // If true, display only a list of usernames and count of
    // the users logged on.
    // Ignored for 'who am i'.
    let short_list = matches.get_flag(options::COUNT);

    let all = matches.get_flag(options::ALL);

    // If true, display a line at the top describing each field.
    let include_heading = matches.get_flag(options::HEADING);

    // If true, display a '+' for each user if mesg y, a '-' if mesg n,
    // or a '?' if their tty cannot be statted.
    let include_mesg = all || matches.get_flag(options::MESG);

    // If true, display the last boot time.
    let need_boottime = all || matches.get_flag(options::BOOT);

    // If true, display dead processes.
    let need_deadprocs = all || matches.get_flag(options::DEAD);

    // If true, display processes waiting for user login.
    let need_login = all || matches.get_flag(options::LOGIN);

    // If true, display processes started by init.
    let need_initspawn = all || matches.get_flag(options::PROCESS);

    // If true, display the last clock change.
    let need_clockchange = all || matches.get_flag(options::TIME);

    // If true, display the current runlevel.
    let need_runlevel = all || matches.get_flag(options::RUNLEVEL);

    let use_defaults = !(all
        || need_boottime
        || need_deadprocs
        || need_login
        || need_initspawn
        || need_runlevel
        || need_clockchange
        || matches.get_flag(options::USERS));

    // If true, display user processes.
    let need_users = all || matches.get_flag(options::USERS) || use_defaults;

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or "." if within the last minute, or "old" if
    // not within the last day.
    let include_idle = need_deadprocs || need_login || need_runlevel || need_users;

    // If true, display process termination & exit status.
    let include_exit = need_deadprocs;

    // If true, display only name, line, and time fields.
    let short_output = !include_exit && use_defaults;

    // If true, display info only for the controlling tty.
    let my_line_only = matches.get_flag(options::ONLY_HOSTNAME_USER) || files.len() == 2;

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

    who.exec()
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL)
                .long(options::ALL)
                .short('a')
                .help("same as -b -d --login -p -r -t -T -u")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::BOOT)
                .long(options::BOOT)
                .short('b')
                .help("time of last system boot")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DEAD)
                .long(options::DEAD)
                .short('d')
                .help("print dead processes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HEADING)
                .long(options::HEADING)
                .short('H')
                .help("print line of column headings")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOGIN)
                .long(options::LOGIN)
                .short('l')
                .help("print system login processes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::LOOKUP)
                .long(options::LOOKUP)
                .help("attempt to canonicalize hostnames via DNS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_HOSTNAME_USER)
                .short('m')
                .help("only hostname and user associated with stdin")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PROCESS)
                .long(options::PROCESS)
                .short('p')
                .help("print active processes spawned by init")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COUNT)
                .long(options::COUNT)
                .short('q')
                .help("all login names and number of users logged on")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RUNLEVEL)
                .long(options::RUNLEVEL)
                .short('r')
                .help(RUNLEVEL_HELP)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHORT)
                .long(options::SHORT)
                .short('s')
                .help("print only name, line, and time (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .short('t')
                .help("print last system clock change")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USERS)
                .long(options::USERS)
                .short('u')
                .help("list users logged in")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MESG)
                .long(options::MESG)
                .short('T')
                .visible_short_alias('w')
                .visible_aliases(&["message", "writable"])
                .help("add user's message status as +, - or ?")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .num_args(1..=2)
                .value_hint(clap::ValueHint::FilePath),
        )
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
        static NOW: time::OffsetDateTime = time::OffsetDateTime::now_local().unwrap();
    }
    NOW.with(|n| {
        let now = n.unix_timestamp();
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
    // "%b %e %H:%M"
    let time_format: Vec<time::format_description::FormatItem> =
        time::format_description::parse("[month repr:short] [day padding:space] [hour]:[minute]")
            .unwrap();
    ut.login_time().format(&time_format).unwrap() // LC_ALL=C
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
    fn exec(&mut self) -> UResult<()> {
        let run_level_chk = |_record: i16| {
            #[cfg(not(target_os = "linux"))]
            return false;

            #[cfg(target_os = "linux")]
            return _record == utmpx::RUN_LVL;
        };

        let f = if self.args.len() == 1 {
            self.args[0].as_ref()
        } else {
            utmpx::DEFAULT_FILE
        };
        if self.short_list {
            let users = Utmpx::iter_all_records_from(f)
                .filter(Utmpx::is_user_process)
                .map(|ut| ut.user())
                .collect::<Vec<_>>();
            println!("{}", users.join(" "));
            println!("# users={}", users.len());
        } else {
            let records = Utmpx::iter_all_records_from(f).peekable();

            if self.include_heading {
                self.print_heading();
            }
            let cur_tty = if self.my_line_only {
                current_tty()
            } else {
                "".to_owned()
            };

            for ut in records {
                if !self.my_line_only || cur_tty == ut.tty_device() {
                    if self.need_users && ut.is_user_process() {
                        self.print_user(&ut)?;
                    } else if self.need_runlevel && run_level_chk(ut.record_type()) {
                        if cfg!(target_os = "linux") {
                            self.print_runlevel(&ut);
                        }
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
        Ok(())
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

    fn print_user(&self, ut: &Utmpx) -> UResult<()> {
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

        let s = if self.do_lookup {
            ut.canon_host().map_err_context(|| {
                let host = ut.host();
                format!(
                    "failed to canonicalize {}",
                    host.split(':').next().unwrap_or(&host).quote()
                )
            })?
        } else {
            ut.host()
        };
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

        Ok(())
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

        write!(buf, "{:<8}", user).unwrap();
        if self.include_mesg {
            buf.push_str(&msg);
        }
        write!(buf, " {:<12}", line).unwrap();
        // "%b %e %H:%M" (LC_ALL=C)
        let time_size = 3 + 2 + 2 + 1 + 2;
        write!(buf, " {:<1$}", time, time_size).unwrap();

        if !self.short_output {
            if self.include_idle {
                write!(buf, " {:<6}", idle).unwrap();
            }
            write!(buf, " {:>10}", pid).unwrap();
        }
        write!(buf, " {:<8}", comment).unwrap();
        if self.include_exit {
            write!(buf, " {:<12}", exit).unwrap();
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
