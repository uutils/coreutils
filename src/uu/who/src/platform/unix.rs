// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname hostnames runlevel mesg wtmp

use crate::options;
use crate::uu_app;

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::libc::S_IWGRP;
use uucore::translate;

use uucore::utmpx::{self, UtmpxRecord, time};

use std::borrow::Cow;
use std::fmt::Write;
use std::io::{Write as _, stdout};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

fn get_long_usage() -> String {
    translate!("who-long-usage", "default_file" => utmpx::DEFAULT_FILE)
}

/// Which kinds of accounting record are worth reporting.
#[derive(Default)]
struct Selection {
    /// The record left by the last system boot.
    boot: bool,
    /// The records of processes that have since exited.
    exited: bool,
    /// The login processes still waiting for someone to sign in.
    login_slots: bool,
    /// The processes that init spawned.
    init_children: bool,
    /// The record left by the most recent clock adjustment.
    clock_change: bool,
    /// The record holding the current runlevel.
    runlevel: bool,
    /// Ordinary user sessions.
    sessions: bool,
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
struct Layout {
    /// Prepend a header row naming the columns.
    header: bool,
    /// The column reporting whether the terminal accepts messages: `+` when it
    /// does, `-` when it does not, `?` when the terminal cannot be queried.
    write_state: bool,
    /// How long the terminal has been quiet.
    idle: bool,
    /// How the process ended and with what status.
    exit: bool,
    /// Drop everything but the name, line and time columns.
    terse: bool,
}

/// The events that are reported from something other than a live session.
#[derive(Clone, Copy)]
enum Event {
    Boot,
    ClockChange,
    #[cfg(target_os = "linux")]
    Runlevel,
    LoginSlot,
    InitChild,
    Exited,
}

/// One output line, before the columns are padded out.
struct Row<'a> {
    user: &'a str,
    write_state: char,
    line: &'a str,
    time: &'a str,
    idle: &'a str,
    pid: &'a str,
    note: &'a str,
    exit: &'a str,
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

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches =
        uucore::clap_localization::handle_clap_result(uu_app().after_help(get_long_usage()), args)?;

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

struct Who {
    resolve_hosts: bool,
    names_only: bool,
    own_terminal_only: bool,
    select: Selection,
    layout: Layout,
    args: Vec<String>,
}

/// Render how long a terminal has been quiet: `hours:minutes`, `.` when under a
/// minute, and the localized `old` past a day or before the given boot time.
fn format_idle<'a>(when: i64, since_boot: i64) -> Cow<'a, str> {
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

fn format_timestamp(ut: &UtmpxRecord) -> String {
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
    ut.login_time().format(&pattern).unwrap()
}

fn current_tty() -> String {
    rustix::termios::ttyname(std::io::stdin(), Vec::with_capacity(16))
        .map(|s| s.to_string_lossy().trim_start_matches("/dev/").to_owned())
        .unwrap_or_default()
}

impl Who {
    fn exec(&mut self) -> UResult<()> {
        let f = if self.args.len() == 1 {
            self.args[0].as_ref()
        } else {
            utmpx::DEFAULT_FILE
        };
        if self.names_only {
            return self.emit_names(f);
        }

        let records = utmpx::Utmpx::iter_all_records_from(f);

        if self.layout.header {
            self.emit_header()?;
        }
        let cur_tty = if self.own_terminal_only {
            current_tty()
        } else {
            String::new()
        };

        for ut in records {
            if self.own_terminal_only && cur_tty != ut.tty_device() {
                continue;
            }
            if self.select.sessions && ut.is_user_process() {
                self.emit_session(&ut)?;
            } else if let Some(event) = self.event_for(&ut) {
                self.emit_event(&ut, event)?;
            }
        }
        Ok(())
    }

    /// The `-q` report: every login name on one line, then the total.
    fn emit_names(&self, path: &str) -> UResult<()> {
        let users = utmpx::Utmpx::iter_all_records_from(path)
            .filter(UtmpxRecord::is_user_process)
            .map(|ut| ut.user())
            .collect::<Vec<_>>();
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

    /// Map a record to the event it stands for, or `None` when that kind was
    /// not selected.
    fn event_for(&self, ut: &UtmpxRecord) -> Option<Event> {
        let rt = ut.record_type();

        #[cfg(target_os = "linux")]
        if self.select.runlevel && rt == utmpx::RUN_LVL {
            return Some(Event::Runlevel);
        }

        match rt {
            utmpx::BOOT_TIME if self.select.boot => Some(Event::Boot),
            utmpx::NEW_TIME if self.select.clock_change => Some(Event::ClockChange),
            utmpx::INIT_PROCESS if self.select.init_children => Some(Event::InitChild),
            utmpx::LOGIN_PROCESS if self.select.login_slots => Some(Event::LoginSlot),
            utmpx::DEAD_PROCESS if self.select.exited => Some(Event::Exited),
            _ => None,
        }
    }

    fn emit_event(&self, ut: &UtmpxRecord, event: Event) -> UResult<()> {
        let time = format_timestamp(ut);
        let pid = format!("{}", ut.pid());
        let note = translate!("who-login-id", "id" => ut.terminal_suffix());

        // Held outside the match so the borrows below outlive it.
        #[cfg(target_os = "linux")]
        let runlevel_line;
        #[cfg(target_os = "linux")]
        let runlevel_note;
        let exit;

        let row = match event {
            Event::Boot => Row {
                line: &translate!("who-system-boot"),
                time: &time,
                ..Row::default()
            },
            Event::ClockChange => Row {
                line: &translate!("who-clock-change"),
                time: &time,
                ..Row::default()
            },
            #[cfg(target_os = "linux")]
            Event::Runlevel => {
                let last = (ut.pid() / 256) as u8 as char;
                let level = (ut.pid() % 256) as u8 as char;
                runlevel_line = translate!("who-runlevel", "level" => level);
                runlevel_note = translate!("who-runlevel-last", "last" => (if last == 'N' { 'S' } else { 'N' }));
                Row {
                    line: &runlevel_line,
                    time: &time,
                    note: if last.is_control() {
                        ""
                    } else {
                        &runlevel_note
                    },
                    ..Row::default()
                }
            }
            Event::LoginSlot => Row {
                user: &translate!("who-login"),
                line: &ut.tty_device(),
                time: &time,
                pid: &pid,
                note: &note,
                ..Row::default()
            },
            Event::InitChild => Row {
                line: &ut.tty_device(),
                time: &time,
                pid: &pid,
                note: &note,
                ..Row::default()
            },
            Event::Exited => {
                let e = ut.exit_status();
                exit = translate!("who-dead-exit-status", "term" => e.0, "exit" => e.1);
                Row {
                    line: &ut.tty_device(),
                    time: &time,
                    pid: &pid,
                    note: &note,
                    exit: &exit,
                    ..Row::default()
                }
            }
        };

        self.emit_row(&row)
    }

    fn emit_session(&self, ut: &UtmpxRecord) -> UResult<()> {
        let mut p = PathBuf::from("/dev");
        p.push(ut.tty_device().as_str());
        // A terminal that cannot be stat'ed reports an unknown write state and
        // an unknown idle time rather than failing the whole listing.
        let (write_state, last_touched) = match p.metadata() {
            Ok(meta) => {
                #[cfg(all(
                    not(target_vendor = "apple"),
                    not(target_os = "android"),
                    not(target_os = "freebsd")
                ))]
                let iwgrp = S_IWGRP;
                #[cfg(any(target_vendor = "apple", target_os = "android", target_os = "freebsd"))]
                let iwgrp = S_IWGRP as u32;
                let state = if meta.mode() & iwgrp == 0 { '-' } else { '+' };
                (state, meta.atime())
            }
            Err(_) => ('?', 0),
        };

        let idle = if last_touched == 0 {
            "  ?".into()
        } else {
            format_idle(last_touched, 0)
        };

        let host = if self.resolve_hosts {
            ut.canon_host().map_err_context(|| {
                let host = ut.host();
                translate!("who-canonicalize-error", "host" => host.split(':').next().unwrap_or(&host).quote())
            })?
        } else {
            ut.host()
        };
        let note = if host.is_empty() {
            host
        } else {
            format!("({host})")
        };

        self.emit_row(&Row {
            user: &ut.user(),
            write_state,
            line: &ut.tty_device(),
            time: &format_timestamp(ut),
            idle: &idle,
            pid: &format!("{}", ut.pid()),
            note: &note,
            exit: "",
        })?;

        Ok(())
    }

    fn emit_row(&self, row: &Row) -> UResult<()> {
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
    fn emit_header(&self) -> UResult<()> {
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
}
