// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

use uucore::entries::{Locate, Passwd};
use uucore::error::{FromIo, UResult};
use uucore::libc::S_IWGRP;
use uucore::utmpx::{self, time, Utmpx};

use std::io::prelude::*;
use std::io::BufReader;

use std::fs::File;
use std::os::unix::fs::MetadataExt;

use clap::{crate_version, Arg, Command};
use std::path::PathBuf;
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str = "pinky - lightweight finger";
const USAGE: &str = "{} [OPTION]... [USER]...";

mod options {
    pub const LONG_FORMAT: &str = "long_format";
    pub const OMIT_HOME_DIR: &str = "omit_home_dir";
    pub const OMIT_PROJECT_FILE: &str = "omit_project_file";
    pub const OMIT_PLAN_FILE: &str = "omit_plan_file";
    pub const SHORT_FORMAT: &str = "short_format";
    pub const OMIT_HEADINGS: &str = "omit_headings";
    pub const OMIT_NAME: &str = "omit_name";
    pub const OMIT_NAME_HOST: &str = "omit_name_host";
    pub const OMIT_NAME_HOST_TIME: &str = "omit_name_host_time";
    pub const USER: &str = "user";
}

fn get_long_usage() -> String {
    format!(
        "A lightweight 'finger' program;  print user information.\n\
         The utmp file will be {}.",
        utmpx::DEFAULT_FILE
    )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let after_help = get_long_usage();

    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let users: Vec<String> = matches
        .values_of(options::USER)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or blank if within the last minute, or days followed
    // by a 'd' if not within the last day.
    let mut include_idle = true;

    // If true, display a line at the top describing each field.
    let include_heading = !matches.is_present(options::OMIT_HEADINGS);

    // if true, display the user's full name from pw_gecos.
    let mut include_fullname = true;

    // if true, display the user's ~/.project file when doing long format.
    let include_project = !matches.is_present(options::OMIT_PROJECT_FILE);

    // if true, display the user's ~/.plan file when doing long format.
    let include_plan = !matches.is_present(options::OMIT_PLAN_FILE);

    // if true, display the user's home directory and shell
    // when doing long format.
    let include_home_and_shell = !matches.is_present(options::OMIT_HOME_DIR);

    // if true, use the "short" output format.
    let do_short_format = !matches.is_present(options::LONG_FORMAT);

    /* if true, display the ut_host field. */
    let mut include_where = true;

    if matches.is_present(options::OMIT_NAME) {
        include_fullname = false;
    }
    if matches.is_present(options::OMIT_NAME_HOST) {
        include_fullname = false;
        include_where = false;
    }
    if matches.is_present(options::OMIT_NAME_HOST_TIME) {
        include_fullname = false;
        include_idle = false;
        include_where = false;
    }

    let pk = Pinky {
        include_idle,
        include_heading,
        include_fullname,
        include_project,
        include_plan,
        include_home_and_shell,
        include_where,
        names: users,
    };

    if do_short_format {
        match pk.short_pinky() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.map_err_context(String::new)),
        }
    } else {
        pk.long_pinky();
        Ok(())
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::LONG_FORMAT)
                .short('l')
                .requires(options::USER)
                .help("produce long format output for the specified USERs"),
        )
        .arg(
            Arg::new(options::OMIT_HOME_DIR)
                .short('b')
                .help("omit the user's home directory and shell in long format"),
        )
        .arg(
            Arg::new(options::OMIT_PROJECT_FILE)
                .short('h')
                .help("omit the user's project file in long format"),
        )
        .arg(
            Arg::new(options::OMIT_PLAN_FILE)
                .short('p')
                .help("omit the user's plan file in long format"),
        )
        .arg(
            Arg::new(options::SHORT_FORMAT)
                .short('s')
                .help("do short format output, this is the default"),
        )
        .arg(
            Arg::new(options::OMIT_HEADINGS)
                .short('f')
                .help("omit the line of column headings in short format"),
        )
        .arg(
            Arg::new(options::OMIT_NAME)
                .short('w')
                .help("omit the user's full name in short format"),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST)
                .short('i')
                .help("omit the user's full name and remote host in short format"),
        )
        .arg(
            Arg::new(options::OMIT_NAME_HOST_TIME)
                .short('q')
                .help("omit the user's full name, remote host and idle time in short format"),
        )
        .arg(
            Arg::new(options::USER)
                .takes_value(true)
                .multiple_occurrences(true),
        )
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

pub trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for str {
    fn capitalize(&self) -> String {
        self.char_indices()
            .fold(String::with_capacity(self.len()), |mut acc, x| {
                if x.0 != 0 {
                    acc.push(x.1);
                } else {
                    acc.push(x.1.to_ascii_uppercase());
                }
                acc
            })
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

fn time_string(ut: &Utmpx) -> String {
    time::strftime("%b %e %H:%M", &ut.login_time()).unwrap() // LC_ALL=C
}

fn gecos_to_fullname(pw: &Passwd) -> String {
    let mut gecos = pw.user_info.clone();
    if let Some(n) = gecos.find(',') {
        gecos.truncate(n);
    }
    gecos.replace('&', &pw.name.capitalize())
}

impl Pinky {
    fn print_entry(&self, ut: &Utmpx) -> std::io::Result<()> {
        let mut pts_path = PathBuf::from("/dev");
        pts_path.push(ut.tty_device().as_str());

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

        print!("{1:<8.0$}", utmpx::UT_NAMESIZE, ut.user());

        if self.include_fullname {
            if let Ok(pw) = Passwd::locate(ut.user().as_ref()) {
                print!(" {:<19.19}", gecos_to_fullname(&pw));
            } else {
                print!(" {:19}", "        ???");
            }
        }

        print!(" {}{:<8.*}", mesg, utmpx::UT_LINESIZE, ut.tty_device());

        if self.include_idle {
            if last_change != 0 {
                print!(" {:<6}", idle_string(last_change));
            } else {
                print!(" {:<6}", "?????");
            }
        }

        print!(" {}", time_string(ut));

        let mut s = ut.host();
        if self.include_where && !s.is_empty() {
            s = ut.canon_host()?;
            print!(" {}", s);
        }

        println!();
        Ok(())
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
        println!();
    }

    fn short_pinky(&self) -> std::io::Result<()> {
        if self.include_heading {
            self.print_heading();
        }
        for ut in Utmpx::iter_all_records() {
            if ut.is_user_process()
                && (self.names.is_empty() || self.names.iter().any(|n| n.as_str() == ut.user()))
            {
                self.print_entry(&ut)?;
            }
        }
        Ok(())
    }

    fn long_pinky(&self) {
        for u in &self.names {
            print!("Login name: {:<28}In real life: ", u);
            if let Ok(pw) = Passwd::locate(u.as_str()) {
                println!(" {}", gecos_to_fullname(&pw));
                if self.include_home_and_shell {
                    print!("Directory: {:<29}", pw.user_dir);
                    println!("Shell:  {}", pw.user_shell);
                }
                if self.include_project {
                    let mut p = PathBuf::from(&pw.user_dir);
                    p.push(".project");
                    if let Ok(f) = File::open(p) {
                        print!("Project: ");
                        read_to_console(f);
                    }
                }
                if self.include_plan {
                    let mut p = PathBuf::from(&pw.user_dir);
                    p.push(".plan");
                    if let Ok(f) = File::open(p) {
                        println!("Plan:");
                        read_to_console(f);
                    }
                }
                println!();
            } else {
                println!(" ???");
            }
        }
    }
}

fn read_to_console<F: Read>(f: F) {
    let mut reader = BufReader::new(f);
    let mut iobuf = Vec::new();
    if reader.read_to_end(&mut iobuf).is_ok() {
        print!("{}", String::from_utf8_lossy(&iobuf));
    }
}
