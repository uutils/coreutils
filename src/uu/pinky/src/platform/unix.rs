// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

use crate::Capitalize;
use crate::options;
use crate::uu_app;

use uucore::entries::{Locate, Passwd};
use uucore::error::UResult;
use uucore::libc::S_IWGRP;
use uucore::translate;
use uucore::utmpx::{self, Utmpx, UtmpxRecord, time};

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

fn get_long_usage() -> String {
    format!(
        "{}{}",
        translate!("pinky-long-usage-description"),
        utmpx::DEFAULT_FILE
    )
}

/// Which columns the short listing carries.
struct Layout {
    /// Prepend a header row naming the columns.
    header: bool,
    /// The account holder's real name, taken from the password file.
    real_name: bool,
    /// How long the terminal has been quiet.
    idle: bool,
    /// Where the session came from: the host recorded alongside it.
    origin: bool,
}

/// How much of each account the long report spells out.
struct Details {
    /// The home directory and the login shell.
    home_and_shell: bool,
    /// The contents of `~/.project`.
    project: bool,
    /// The contents of `~/.plan`.
    plan: bool,
}

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches =
        uucore::clap_localization::handle_clap_result(uu_app().after_help(get_long_usage()), args)?;

    let users: Vec<String> = matches
        .get_many::<String>(options::USER)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    // The three narrowing flags stack: `-w` omits the real name, `-i` omits
    // the origin as well, and `-q` keeps neither of those nor the idle time.
    let omit_name = matches.get_flag(options::OMIT_NAME);
    let omit_name_host = matches.get_flag(options::OMIT_NAME_HOST);
    let omit_name_host_time = matches.get_flag(options::OMIT_NAME_HOST_TIME);

    let pinky = Pinky {
        // Resolve each recorded host to its canonical name before printing it.
        resolve_hosts: matches.get_flag(options::LOOKUP),
        layout: Layout {
            header: !matches.get_flag(options::OMIT_HEADINGS),
            real_name: !(omit_name || omit_name_host || omit_name_host_time),
            idle: !omit_name_host_time,
            origin: !(omit_name_host || omit_name_host_time),
        },
        details: Details {
            home_and_shell: !matches.get_flag(options::OMIT_HOME_DIR),
            project: !matches.get_flag(options::OMIT_PROJECT_FILE),
            plan: !matches.get_flag(options::OMIT_PLAN_FILE),
        },
        names: users,
    };

    let mut stdout = io::stdout().lock();
    if matches.get_flag(options::LONG_FORMAT) {
        pinky.write_long(&mut stdout)?;
    } else {
        pinky.write_short(&mut stdout)?;
    }
    Ok(())
}

struct Pinky {
    resolve_hosts: bool,
    layout: Layout,
    details: Details,
    names: Vec<String>,
}

/// Render how long a terminal has been quiet: blanks under a minute,
/// `hours:minutes` under a day, and a count of days past that.
fn idle_string(when: i64) -> String {
    thread_local! {
        static NOW: time::OffsetDateTime = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    }
    NOW.with(|n| {
        let duration = n.unix_timestamp() - when;
        if duration < 60 {
            "     ".to_owned()
        } else if duration < 24 * 3600 {
            let hours = duration / (60 * 60);
            let minutes = (duration % (60 * 60)) / 60;
            format!("{hours:02}:{minutes:02}")
        } else {
            let days = duration / (24 * 3600);
            format!("{days}d")
        }
    })
}

/// Render an entry's login time. The C locale gets the terse month-and-day
/// form; everything else gets the ISO-like one.
fn time_string(ut: &UtmpxRecord) -> String {
    const FORMAT_DESCRIPTION_VERSION: usize = 2;

    let time_format: Vec<time::format_description::FormatItem> = if ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(std::env::var_os)
        .as_deref()
        == Some(std::ffi::OsStr::new("C"))
    {
        // "%b %e %H:%M"
        time::format_description::parse_borrowed::<FORMAT_DESCRIPTION_VERSION>(
            "[month repr:short] [day padding:space] [hour]:[minute]",
        )
        .unwrap()
    } else {
        // "%Y-%m-%d %H:%M"
        time::format_description::parse_borrowed::<FORMAT_DESCRIPTION_VERSION>(
            "[year]-[month]-[day] [hour]:[minute]",
        )
        .unwrap()
    };
    ut.login_time().format(&time_format).unwrap()
}

/// Pull the real name out of a password entry: it is the part of the comment
/// field ahead of the first comma, with `&` standing in for the login name.
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
    fn write_entry(&self, writer: &mut impl Write, ut: &UtmpxRecord) -> io::Result<()> {
        let mut pts_path = PathBuf::from("/dev");
        pts_path.push(ut.tty_device().as_str());

        let mesg;
        let last_change;

        #[allow(clippy::unnecessary_cast)]
        if let Ok(meta) = pts_path.metadata() {
            mesg = if (meta.mode() & (S_IWGRP as u32)) == 0 {
                '*'
            } else {
                ' '
            };
            last_change = meta.atime();
        } else {
            mesg = '?';
            last_change = 0;
        }

        write!(writer, "{1:<8.0$}", utmpx::UT_NAMESIZE, ut.user())?;

        if self.layout.real_name {
            let fullname = if let Ok(pw) = Passwd::locate(ut.user().as_ref()) {
                gecos_to_fullname(&pw)
            } else {
                None
            };
            if let Some(fullname) = fullname {
                write!(writer, " {fullname:<19.19}")?;
            } else {
                write!(writer, " {:19}", "        ???")?;
            }
        }

        write!(
            writer,
            " {mesg}{:<8.*}",
            utmpx::UT_LINESIZE,
            ut.tty_device()
        )?;

        if self.layout.idle {
            if last_change == 0 {
                write!(writer, " {:<6}", "?????")?;
            } else {
                write!(writer, " {:<6}", idle_string(last_change))?;
            }
        }

        write!(writer, " {}", time_string(ut))?;

        if self.layout.origin {
            let s: String = if self.resolve_hosts {
                ut.canon_host().unwrap_or(ut.host())
            } else {
                ut.host()
            };

            if !s.is_empty() {
                write!(writer, " {s}")?;
            }
        }

        writeln!(writer)?;
        Ok(())
    }

    fn write_heading(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "{:<8}", translate!("pinky-column-login"))?;
        if self.layout.real_name {
            write!(writer, " {:<19}", translate!("pinky-column-name"))?;
        }
        write!(writer, " {:<9}", translate!("pinky-column-tty"))?;
        if self.layout.idle {
            write!(writer, " {:<6}", translate!("pinky-column-idle"))?;
        }
        write!(writer, " {:<16}", translate!("pinky-column-when"))?;
        if self.layout.origin {
            write!(writer, " {}", translate!("pinky-column-where"))?;
        }
        writeln!(writer)?;
        Ok(())
    }

    fn write_short(&self, writer: &mut impl Write) -> io::Result<()> {
        if self.layout.header {
            self.write_heading(writer)?;
        }
        for ut in Utmpx::iter_all_records() {
            if ut.is_user_process()
                && (self.names.is_empty() || self.names.iter().any(|n| n.as_str() == ut.user()))
            {
                self.write_entry(writer, &ut)?;
            }
        }
        Ok(())
    }

    fn write_long(&self, writer: &mut impl Write) -> io::Result<()> {
        for u in &self.names {
            write!(
                writer,
                "{} {u:<28}{} ",
                translate!("pinky-login-name-label"),
                translate!("pinky-real-life-label")
            )?;
            if let Ok(pw) = Passwd::locate(u.as_str()) {
                let fullname = gecos_to_fullname(&pw).unwrap_or_default();
                let user_dir = pw.user_dir.unwrap_or_default();
                let user_shell = pw.user_shell.unwrap_or_default();
                writeln!(writer, " {fullname}")?;
                if self.details.home_and_shell {
                    write!(
                        writer,
                        "{} {user_dir:<29}",
                        translate!("pinky-directory-label")
                    )?;
                    writeln!(writer, "{}  {user_shell}", translate!("pinky-shell-label"))?;
                }
                if self.details.project {
                    let mut p = PathBuf::from(&user_dir);
                    p.push(".project");
                    if let Ok(mut reader) = File::open(p) {
                        write!(writer, "{} ", translate!("pinky-project-label"))?;
                        io::copy(&mut reader, writer)?;
                    }
                }
                if self.details.plan {
                    let mut p = PathBuf::from(&user_dir);
                    p.push(".plan");
                    if let Ok(mut reader) = File::open(p) {
                        writeln!(writer, "{}:", translate!("pinky-plan-label"))?;
                        io::copy(&mut reader, writer)?;
                    }
                }
                writeln!(writer)?;
            } else {
                writeln!(writer, " ???")?;
            }
        }
        Ok(())
    }
}
