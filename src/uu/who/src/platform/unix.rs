// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname hostnames runlevel mesg wtmp

use crate::{Row, Who, format_idle, format_timestamp};

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::libc::S_IWGRP;
use uucore::translate;

use uucore::utmpx::{self, UtmpxRecord};

use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub(crate) fn get_long_usage() -> String {
    translate!("who-long-usage", "default_file" => utmpx::DEFAULT_FILE)
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
fn current_tty() -> String {
    rustix::termios::ttyname(std::io::stdin(), Vec::with_capacity(16))
        .map(|s| s.to_string_lossy().trim_start_matches("/dev/").to_owned())
        .unwrap_or_default()
}

impl Who {
    pub(crate) fn exec(&mut self) -> UResult<()> {
        let f = if self.args.len() == 1 {
            self.args[0].as_ref()
        } else {
            utmpx::DEFAULT_FILE
        };
        if self.names_only {
            let users = utmpx::Utmpx::iter_all_records_from(f)
                .filter(UtmpxRecord::is_user_process)
                .map(|ut| ut.user())
                .collect::<Vec<_>>();
            return self.emit_names(&users);
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
        let time = format_timestamp(ut.login_time());
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
            time: &format_timestamp(ut.login_time()),
            idle: &idle,
            pid: &format!("{}", ut.pid()),
            note: &note,
            exit: "",
        })?;

        Ok(())
    }
}
