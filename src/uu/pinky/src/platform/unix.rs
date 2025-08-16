// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

use crate::Capitalize;
use crate::options;
use crate::uu_app;

use uucore::LocalizedCommand;
use uucore::entries::{Locate, Passwd};
use uucore::error::{FromIo, UResult};
use uucore::libc::S_IWGRP;
use uucore::translate;
use uucore::utmpx::{self, Utmpx, UtmpxRecord, time};

use std::io::BufReader;
use std::io::prelude::*;

use std::fs::File;
use std::os::unix::fs::MetadataExt;

use std::path::PathBuf;

fn get_long_usage() -> String {
    format!(
        "{}{}",
        translate!("pinky-long-usage-description"),
        utmpx::DEFAULT_FILE
    )
}

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_long_usage())
        .get_matches_from_localized(args);

    let users: Vec<String> = matches
        .get_many::<String>(options::USER)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    // If true, display the hours:minutes since each user has touched
    // the keyboard, or blank if within the last minute, or days followed
    // by a 'd' if not within the last day.
    let mut include_idle = true;

    // If true, display a line at the top describing each field.
    let include_heading = !matches.get_flag(options::OMIT_HEADINGS);

    // if true, display the user's full name from pw_gecos.
    let mut include_fullname = true;

    // if true, display the user's ~/.project file when doing long format.
    let include_project = !matches.get_flag(options::OMIT_PROJECT_FILE);

    // if true, display the user's ~/.plan file when doing long format.
    let include_plan = !matches.get_flag(options::OMIT_PLAN_FILE);

    // if true, display the user's home directory and shell
    // when doing long format.
    let include_home_and_shell = !matches.get_flag(options::OMIT_HOME_DIR);

    // if true, use the "short" output format.
    let do_short_format = !matches.get_flag(options::LONG_FORMAT);

    /* if true, display the ut_host field. */
    let mut include_where = true;

    if matches.get_flag(options::OMIT_NAME) {
        include_fullname = false;
    }
    if matches.get_flag(options::OMIT_NAME_HOST) {
        include_fullname = false;
        include_where = false;
    }
    if matches.get_flag(options::OMIT_NAME_HOST_TIME) {
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

fn idle_string(when: i64) -> String {
    thread_local! {
        static NOW: time::OffsetDateTime = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    }
    NOW.with(|n| {
        let duration = n.unix_timestamp() - when;
        if duration < 60 {
            // less than 1min
            "     ".to_owned()
        } else if duration < 24 * 3600 {
            // less than 1day
            let hours = duration / (60 * 60);
            let minutes = (duration % (60 * 60)) / 60;
            format!("{hours:02}:{minutes:02}")
        } else {
            // more than 1day
            let days = duration / (24 * 3600);
            format!("{days}d")
        }
    })
}

fn time_string(ut: &UtmpxRecord) -> String {
    // "%b %e %H:%M"
    let time_format: Vec<time::format_description::FormatItem> =
        time::format_description::parse("[month repr:short] [day padding:space] [hour]:[minute]")
            .unwrap();
    ut.login_time().format(&time_format).unwrap() // LC_ALL=C
}

fn gecos_to_fullname(pw: &Passwd) -> Option<String> {
    let mut gecos = if let Some(gecos) = &pw.user_info {
        gecos.clone()
    } else {
        return None;
    };
    if let Some(n) = gecos.find(',') {
        gecos.truncate(n);
    }
    Some(gecos.replace('&', &pw.name.capitalize()))
}

impl Pinky {
    fn print_entry(&self, ut: &UtmpxRecord) -> std::io::Result<()> {
        let mut pts_path = PathBuf::from("/dev");
        pts_path.push(ut.tty_device().as_str());

        let mesg;
        let last_change;

        match pts_path.metadata() {
            #[allow(clippy::unnecessary_cast)]
            Ok(meta) => {
                mesg = if meta.mode() & S_IWGRP as u32 == 0 {
                    '*'
                } else {
                    ' '
                };
                last_change = meta.atime();
            }
            _ => {
                mesg = ' ';
                last_change = 0;
            }
        }

        print!("{1:<8.0$}", utmpx::UT_NAMESIZE, ut.user());

        if self.include_fullname {
            let fullname = if let Ok(pw) = Passwd::locate(ut.user().as_ref()) {
                gecos_to_fullname(&pw)
            } else {
                None
            };
            if let Some(fullname) = fullname {
                print!(" {fullname:<19.19}");
            } else {
                print!(" {:19}", "        ???");
            }
        }

        print!(" {mesg}{:<8.*}", utmpx::UT_LINESIZE, ut.tty_device());

        if self.include_idle {
            if last_change == 0 {
                print!(" {:<6}", "?????");
            } else {
                print!(" {:<6}", idle_string(last_change));
            }
        }

        print!(" {}", time_string(ut));

        let mut s = ut.host();
        if self.include_where && !s.is_empty() {
            s = ut.canon_host()?;
            print!(" {s}");
        }

        println!();
        Ok(())
    }

    fn print_heading(&self) {
        print!("{:<8}", translate!("pinky-column-login"));
        if self.include_fullname {
            print!(" {:<19}", translate!("pinky-column-name"));
        }
        print!(" {:<9}", translate!("pinky-column-tty"));
        if self.include_idle {
            print!(" {:<6}", translate!("pinky-column-idle"));
        }
        print!(" {:<16}", translate!("pinky-column-when"));
        if self.include_where {
            print!(" {}", translate!("pinky-column-where"));
        }
        println!();
    }

    fn short_pinky(&self) -> std::io::Result<()> {
        if self.include_heading {
            self.print_heading();
        }

        let mut records: Vec<_> = Utmpx::iter_all_records()
            .filter(|ut| {
                ut.is_user_process()
                    && (self.names.is_empty() || self.names.iter().any(|n| n.as_str() == ut.user()))
            })
            .collect();

        // Sort by TTY device name to match GNU pinky's output order.
        records.sort_by_key(|ut| ut.tty_device());

        for ut in records {
            self.print_entry(&ut)?;
        }
        Ok(())
    }

    fn long_pinky(&self) {
        for u in &self.names {
            print!(
                "{} {u:<28}{} ",
                translate!("pinky-login-name-label"),
                translate!("pinky-real-life-label")
            );
            if let Ok(pw) = Passwd::locate(u.as_str()) {
                let fullname = gecos_to_fullname(&pw).unwrap_or_default();
                let user_dir = pw.user_dir.unwrap_or_default();
                let user_shell = pw.user_shell.unwrap_or_default();
                println!(" {fullname}");
                if self.include_home_and_shell {
                    print!("{} {user_dir:<29}", translate!("pinky-directory-label"));
                    println!("{}  {user_shell}", translate!("pinky-shell-label"));
                }
                if self.include_project {
                    let mut p = PathBuf::from(&user_dir);
                    p.push(".project");
                    if let Ok(f) = File::open(p) {
                        print!("{} ", translate!("pinky-project-label"));
                        read_to_console(f);
                    }
                }
                if self.include_plan {
                    let mut p = PathBuf::from(&user_dir);
                    p.push(".plan");
                    if let Ok(f) = File::open(p) {
                        println!("{}:", translate!("pinky-plan-label"));
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
