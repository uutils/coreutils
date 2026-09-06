// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::capitalize;
use crate::options;
use crate::uu_app;

use uucore::entries::{Locate, Passwd};
use uucore::error::UResult;
use uucore::libc::S_IWGRP;
use uucore::translate;
use uucore::utmpx::{self, Utmpx, UtmpxRecord, time};

use std::fmt::{self, Write as _};
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

/// Whether the terminal would let another user write to it, which is what
/// the flag ahead of the device name in the short listing reports.
enum Messages {
    /// The device is group-writable, so `write` and `wall` can reach it.
    Accepted,
    /// The device is not group-writable; `mesg n` leaves a terminal here.
    Refused,
    /// The device could not be stat'd, so there is nothing to report.
    Unknown,
}

impl fmt::Display for Messages {
    /// The listing spends one column on this, the same character GNU prints.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Accepted => " ",
            Self::Refused => "*",
            Self::Unknown => "?",
        })
    }
}

/// What an entry's terminal has to say about the session.
struct Terminal {
    messages: Messages,
    /// When the terminal was last read from, if that is known.
    read_at: Option<i64>,
}

impl Terminal {
    /// Query the device the entry names, relative to `/dev`.
    fn query(device: &str) -> Self {
        let mut path = PathBuf::from("/dev");
        path.push(device);

        #[allow(clippy::unnecessary_cast)]
        match path.metadata() {
            Ok(meta) => Self {
                messages: if meta.mode() & (S_IWGRP as u32) == 0 {
                    Messages::Refused
                } else {
                    Messages::Accepted
                },
                read_at: Some(meta.atime()).filter(|at| *at != 0),
            },
            Err(_) => Self {
                messages: Messages::Unknown,
                read_at: None,
            },
        }
    }
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
        pinky.describe_users(&mut stdout)?;
    } else {
        pinky.list_sessions(&mut stdout)?;
    }
    Ok(())
}

/// Render how long the terminal last read at `read_at` has been quiet.
fn format_idle(read_at: i64) -> String {
    thread_local! {
        static NOW: time::OffsetDateTime = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    }
    NOW.with(|n| format_quiet_time(n.unix_timestamp() - read_at))
}

/// Render `quiet_for` seconds of silence: blanks under a minute,
/// `hours:minutes` under a day, and a count of days past that.
fn format_quiet_time(quiet_for: i64) -> String {
    if quiet_for < 60 {
        "     ".to_owned()
    } else if quiet_for < 24 * 3600 {
        format!("{:02}:{:02}", quiet_for / 3600, (quiet_for % 3600) / 60)
    } else {
        format!("{}d", quiet_for / (24 * 3600))
    }
}

/// Render an entry's login time.
fn format_timestamp(ut: &UtmpxRecord) -> String {
    const FORMAT_DESCRIPTION_VERSION: usize = 2;

    thread_local! {
        /// The C locale gets the terse month-and-day form; everything else gets
        /// the ISO-like one. Neither the environment nor the descriptions change
        /// while we print, so settle on one and parse it once.
        static TIME_FORMAT: Vec<time::format_description::FormatItem<'static>> = {
            let description = if ["LC_ALL", "LC_TIME", "LANG"]
                .into_iter()
                .find_map(std::env::var_os)
                .as_deref()
                == Some(std::ffi::OsStr::new("C"))
            {
                // "%b %e %H:%M"
                "[month repr:short] [day padding:space] [hour]:[minute]"
            } else {
                // "%Y-%m-%d %H:%M"
                "[year]-[month]-[day] [hour]:[minute]"
            };
            time::format_description::parse_borrowed::<FORMAT_DESCRIPTION_VERSION>(description)
                .unwrap()
        };
    }

    TIME_FORMAT.with(|description| ut.login_time().format(description).unwrap())
}

/// Pull the real name out of a password entry: it is the part of the comment
/// field ahead of the first comma, with `&` standing in for the login name.
fn real_name(pw: &Passwd) -> Option<String> {
    let mut comment = pw.user_info.clone()?;
    if let Some(comma) = comment.find(',') {
        comment.truncate(comma);
    }
    Some(comment.replace('&', &capitalize(&pw.name)))
}

/// Append one column to a row under construction. Only a `Display` impl that
/// fails can fail here, since the sink is a `String`, and no column formats
/// anything but strings and numbers — so say so rather than truncate quietly.
macro_rules! column {
    ($row:expr, $($arg:tt)*) => {{
        write!($row, $($arg)*).expect("a column cannot fail to format");
    }};
}

struct Pinky {
    resolve_hosts: bool,
    layout: Layout,
    details: Details,
    names: Vec<String>,
}

impl Pinky {
    /// Assemble one short-format line, column by column.
    fn session_row(&self, ut: &UtmpxRecord) -> String {
        let terminal = Terminal::query(ut.tty_device().as_str());
        let mut row = String::new();

        column!(row, "{1:<8.0$}", utmpx::UT_NAMESIZE, ut.user());

        if self.layout.real_name {
            let name = Passwd::locate(ut.user().as_ref())
                .ok()
                .and_then(|pw| real_name(&pw));
            match name {
                Some(name) => {
                    column!(row, " {name:<19.19}");
                }
                None => {
                    column!(row, " {:19}", "        ???");
                }
            }
        }

        column!(
            row,
            " {}{:<8.*}",
            terminal.messages,
            utmpx::UT_LINESIZE,
            ut.tty_device()
        );

        if self.layout.idle {
            let idle = match terminal.read_at {
                Some(read_at) => format_idle(read_at),
                None => "?????".to_owned(),
            };
            column!(row, " {idle:<6}");
        }

        column!(row, " {}", format_timestamp(ut));

        if self.layout.origin {
            let host = if self.resolve_hosts {
                ut.canon_host().unwrap_or(ut.host())
            } else {
                ut.host()
            };
            if !host.is_empty() {
                column!(row, " {host}");
            }
        }

        row
    }

    /// Name each column that `Layout` turned on.
    fn header_row(&self) -> String {
        let mut row = String::new();

        column!(row, "{:<8}", translate!("pinky-column-login"));
        if self.layout.real_name {
            column!(row, " {:<19}", translate!("pinky-column-name"));
        }
        column!(row, " {:<9}", translate!("pinky-column-tty"));
        if self.layout.idle {
            column!(row, " {:<6}", translate!("pinky-column-idle"));
        }
        column!(row, " {:<16}", translate!("pinky-column-when"));
        if self.layout.origin {
            column!(row, " {}", translate!("pinky-column-where"));
        }

        row
    }

    /// One line per session, restricted to the named accounts when any were
    /// given on the command line.
    fn list_sessions(&self, writer: &mut impl Write) -> io::Result<()> {
        if self.layout.header {
            writeln!(writer, "{}", self.header_row())?;
        }
        for ut in Utmpx::iter_all_records() {
            if ut.is_user_process()
                && (self.names.is_empty() || self.names.iter().any(|n| n.as_str() == ut.user()))
            {
                writeln!(writer, "{}", self.session_row(&ut))?;
            }
        }
        Ok(())
    }

    /// A paragraph per named account, drawn from the password file and from
    /// the dot files in the account's home directory.
    fn describe_users(&self, writer: &mut impl Write) -> io::Result<()> {
        for name in &self.names {
            write!(
                writer,
                "{} {name:<28}{} ",
                translate!("pinky-login-name-label"),
                translate!("pinky-real-life-label")
            )?;

            // An unknown account gets the header line and nothing else, not
            // even the blank line that closes a paragraph below: GNU returns
            // from the entry right here.
            let Ok(pw) = Passwd::locate(name.as_str()) else {
                writeln!(writer, " ???")?;
                continue;
            };

            writeln!(writer, " {}", real_name(&pw).unwrap_or_default())?;

            let home = pw.user_dir.unwrap_or_default();
            if self.details.home_and_shell {
                write!(writer, "{} {home:<29}", translate!("pinky-directory-label"))?;
                writeln!(
                    writer,
                    "{}  {}",
                    translate!("pinky-shell-label"),
                    pw.user_shell.unwrap_or_default()
                )?;
            }

            if self.details.project
                && let Ok(mut reader) = File::open(PathBuf::from(&home).join(".project"))
            {
                write!(writer, "{} ", translate!("pinky-project-label"))?;
                io::copy(&mut reader, writer)?;
            }
            if self.details.plan
                && let Ok(mut reader) = File::open(PathBuf::from(&home).join(".plan"))
            {
                writeln!(writer, "{}:", translate!("pinky-plan-label"))?;
                io::copy(&mut reader, writer)?;
            }

            writeln!(writer)?;
        }
        Ok(())
    }
}
