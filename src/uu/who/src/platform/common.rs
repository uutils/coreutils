// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) runlevel mesg

use crate::{options, uu_app};

use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::{Write as _, stdout};
use uucore::error::UResult;
use uucore::translate;

/// Which kinds of accounting record are worth reporting.
#[derive(Default)]
pub(crate) struct Selection {
    /// The record left by the last system boot.
    pub(crate) boot: bool,
    /// The records of processes that have since exited.
    pub(crate) exited: bool,
    /// The login processes still waiting for someone to sign in.
    pub(crate) login_slots: bool,
    /// The processes that init spawned.
    pub(crate) init_children: bool,
    /// The record left by the most recent clock adjustment.
    pub(crate) clock_change: bool,
    /// The record holding the current runlevel.
    pub(crate) runlevel: bool,
    /// Ordinary user sessions.
    pub(crate) sessions: bool,
}

impl Selection {
    /// True when no selecting option was given at all, including `--users`.
    /// Such an invocation falls back to reporting user sessions.
    fn is_default(&self) -> bool {
        !(self.boot
            || self.exited
            || self.login_slots
            || self.init_children
            || self.clock_change
            || self.runlevel
            || self.sessions)
    }
}

/// Which columns each row carries.
#[derive(Default)]
pub(crate) struct Layout {
    /// Prepend a header row naming the columns.
    pub(crate) header: bool,
    /// The column reporting whether the terminal accepts messages: `+` when it
    /// does, `-` when it does not, `?` when the terminal cannot be queried.
    pub(crate) write_state: bool,
    /// How long the terminal has been quiet.
    pub(crate) idle: bool,
    /// How the process ended and with what status.
    pub(crate) exit: bool,
    /// Drop everything but the name, line and time columns.
    pub(crate) terse: bool,
}

/// One output line, before the columns are padded out.
pub(crate) struct Row<'a> {
    pub(crate) user: &'a str,
    pub(crate) write_state: char,
    pub(crate) line: &'a str,
    pub(crate) time: &'a str,
    pub(crate) idle: &'a str,
    pub(crate) pid: &'a str,
    pub(crate) note: &'a str,
    pub(crate) exit: &'a str,
}

impl Default for Row<'_> {
    fn default() -> Self {
        Self {
            user: "",
            write_state: ' ',
            line: "",
            time: "",
            idle: "",
            pid: "",
            note: "",
            exit: "",
        }
    }
}

pub struct Who {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) all: bool,
    pub(crate) resolve_hosts: bool,
    pub(crate) names_only: bool,
    pub(crate) own_terminal_only: bool,
    pub(crate) select: Selection,
    pub(crate) layout: Layout,
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(crate) args: Vec<String>,
}

impl Who {
    pub(crate) fn emit_row(&self, row: &Row) -> UResult<()> {
        // Width of "%b %e %H:%M" under LC_ALL=C.
        const TIME_WIDTH: usize = 3 + 2 + 2 + 1 + 2;

        let mut buf = String::with_capacity(64);
        write!(buf, "{:<8}", row.user).unwrap();
        if self.layout.write_state {
            buf.push(' ');
            buf.push(row.write_state);
        }
        write!(buf, " {:<12}", row.line).unwrap();
        write!(buf, " {:<TIME_WIDTH$}", row.time).unwrap();

        if !self.layout.terse {
            if self.layout.idle {
                write!(buf, " {:<6}", row.idle).unwrap();
            }
            write!(buf, " {:>10}", row.pid).unwrap();
        }
        write!(buf, " {:<8}", row.note).unwrap();
        if self.layout.exit {
            write!(buf, " {:<12}", row.exit).unwrap();
        }
        writeln!(stdout(), "{}", buf.trim_end())?;
        Ok(())
    }

    #[inline]
    pub(crate) fn emit_header(&self) -> UResult<()> {
        self.emit_row(&Row {
            user: &translate!("who-heading-name"),
            write_state: ' ',
            line: &translate!("who-heading-line"),
            time: &translate!("who-heading-time"),
            idle: &translate!("who-heading-idle"),
            pid: &translate!("who-heading-pid"),
            note: &translate!("who-heading-comment"),
            exit: &translate!("who-heading-exit"),
        })?;
        Ok(())
    }

    pub(crate) fn emit_names(&self, users: &[String]) -> UResult<()> {
        // `println!` panics on a write error; the rest of this file surfaces
        // it through `?` instead so the caller can report it and exit
        // non-zero, matching GNU (#13388).
        writeln!(stdout(), "{}", users.join(" "))?;
        writeln!(
            stdout(),
            "{}",
            translate!("who-user-count", "count" => users.len())
        )?;
        Ok(())
    }
}

/// Render how long a terminal has been quiet: `hours:minutes`, `.` when under a
/// minute, and the localized `old` past a day or before the given boot time.
pub(crate) fn format_idle<'a>(when: i64, since_boot: i64) -> Cow<'a, str> {
    thread_local! {
        static NOW: time::OffsetDateTime = time::OffsetDateTime::now_local().unwrap();
    }
    NOW.with(|n| {
        let now = n.unix_timestamp();
        if since_boot < when && now - 24 * 3600 < when && when <= now {
            let quiet_for = now - when;
            if quiet_for < 60 {
                "  .  ".into()
            } else {
                format!("{:02}:{:02}", quiet_for / 3600, (quiet_for % 3600) / 60).into()
            }
        } else {
            translate!("who-idle-old").into()
        }
    })
}

pub(crate) fn format_timestamp(login_time: time::OffsetDateTime) -> String {
    const FORMAT_DESCRIPTION_VERSION: usize = 2;

    let pattern: Vec<time::format_description::FormatItem> = if ["LC_ALL", "LC_TIME", "LANG"]
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
    login_time.format(&pattern).unwrap()
}

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    #[cfg(unix)]
    let app = uu_app().after_help(super::get_long_usage());
    #[cfg(not(unix))]
    let app = uu_app();

    let matches = uucore::clap_localization::handle_clap_result(app, args)?;

    let files: Vec<String> = matches
        .get_many::<String>(options::FILE)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let all = matches.get_flag(options::ALL);
    let flag = |name: &str| all || matches.get_flag(name);

    let mut select = Selection {
        boot: flag(options::BOOT),
        exited: flag(options::DEAD),
        login_slots: flag(options::LOGIN),
        init_children: flag(options::PROCESS),
        clock_change: flag(options::TIME),
        runlevel: flag(options::RUNLEVEL),
        sessions: matches.get_flag(options::USERS),
    };

    // With no selecting option the report falls back to user sessions, and the
    // narrower row shape that goes with them.
    let defaulted = select.is_default();
    select.sessions |= all || defaulted;

    let layout = Layout {
        header: matches.get_flag(options::HEADING),
        write_state: flag(options::MESG),
        // The idle column is only meaningful for records tied to a terminal.
        idle: select.exited || select.login_slots || select.runlevel || select.sessions,
        exit: select.exited,
        terse: !select.exited && defaulted,
    };

    let mut who = Who {
        all,
        // Resolve each recorded host to its canonical name before printing it.
        resolve_hosts: matches.get_flag(options::LOOKUP),
        // Print just the login names followed by a total, instead of one row
        // per record. Carries no meaning in the `who am i` form.
        names_only: matches.get_flag(options::COUNT),
        // Report only the session attached to the invoking terminal.
        own_terminal_only: matches.get_flag(options::ONLY_HOSTNAME_USER) || files.len() == 2,
        select,
        layout,
        args: files,
    };

    who.exec()?;
    Ok(())
}
