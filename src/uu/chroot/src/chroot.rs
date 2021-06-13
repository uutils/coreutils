// This file is part of the uutils coreutils package.
//
// (c) Vsevolod Velichko <torkvemada@sorokdva.net>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) NEWROOT Userspec pstatus

mod app;

#[macro_use]
extern crate uucore;
use std::ffi::CString;
use std::io::Error;
use std::path::Path;
use std::process::Command;
use uucore::libc::{self, chroot, setgid, setgroups, setuid};
use uucore::{entries, InvalidEncodingHandling};

use crate::app::get_app;
mod options {
    pub const NEWROOT: &str = "newroot";
    pub const USER: &str = "user";
    pub const GROUP: &str = "group";
    pub const GROUPS: &str = "groups";
    pub const USERSPEC: &str = "userspec";
    pub const COMMAND: &str = "command";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = get_app(executable!()).get_matches_from(args);

    let default_shell: &'static str = "/bin/sh";
    let default_option: &'static str = "-i";
    let user_shell = std::env::var("SHELL");

    let newroot: &Path = match matches.value_of(options::NEWROOT) {
        Some(v) => Path::new(v),
        None => crash!(
            1,
            "Missing operand: NEWROOT\nTry '{} --help' for more information.",
            executable!()
        ),
    };

    if !newroot.is_dir() {
        crash!(
            1,
            "cannot change root directory to `{}`: no such directory",
            newroot.display()
        );
    }

    let commands = match matches.values_of(options::COMMAND) {
        Some(v) => v.collect(),
        None => vec![],
    };

    // TODO: refactor the args and command matching
    // See: https://github.com/uutils/coreutils/pull/2365#discussion_r647849967
    let command: Vec<&str> = match commands.len() {
        1 => {
            let shell: &str = match user_shell {
                Err(_) => default_shell,
                Ok(ref s) => s.as_ref(),
            };
            vec![shell, default_option]
        }
        _ => commands,
    };

    set_context(newroot, &matches);

    let pstatus = Command::new(command[0])
        .args(&command[1..])
        .status()
        .unwrap_or_else(|e| crash!(1, "Cannot exec: {}", e));

    if pstatus.success() {
        0
    } else {
        pstatus.code().unwrap_or(-1)
    }
}

fn set_context(root: &Path, options: &clap::ArgMatches) {
    let userspec_str = options.value_of(options::USERSPEC);
    let user_str = options.value_of(options::USER).unwrap_or_default();
    let group_str = options.value_of(options::GROUP).unwrap_or_default();
    let groups_str = options.value_of(options::GROUPS).unwrap_or_default();
    let userspec = match userspec_str {
        Some(u) => {
            let s: Vec<&str> = u.split(':').collect();
            if s.len() != 2 || s.iter().any(|&spec| spec.is_empty()) {
                crash!(1, "invalid userspec: `{}`", u)
            };
            s
        }
        None => Vec::new(),
    };

    let (user, group) = if userspec.is_empty() {
        (user_str, group_str)
    } else {
        (userspec[0], userspec[1])
    };

    enter_chroot(root);

    set_groups_from_str(groups_str);
    set_main_group(group);
    set_user(user);
}

fn enter_chroot(root: &Path) {
    let root_str = root.display();
    std::env::set_current_dir(root).unwrap();
    let err = unsafe {
        chroot(CString::new(".").unwrap().as_bytes_with_nul().as_ptr() as *const libc::c_char)
    };
    if err != 0 {
        crash!(
            1,
            "cannot chroot to {}: {}",
            root_str,
            Error::last_os_error()
        )
    };
}

fn set_main_group(group: &str) {
    if !group.is_empty() {
        let group_id = match entries::grp2gid(group) {
            Ok(g) => g,
            _ => crash!(1, "no such group: {}", group),
        };
        let err = unsafe { setgid(group_id) };
        if err != 0 {
            crash!(
                1,
                "cannot set gid to {}: {}",
                group_id,
                Error::last_os_error()
            )
        }
    }
}

#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
fn set_groups(groups: Vec<libc::gid_t>) -> libc::c_int {
    unsafe { setgroups(groups.len() as libc::c_int, groups.as_ptr()) }
}

#[cfg(target_os = "linux")]
fn set_groups(groups: Vec<libc::gid_t>) -> libc::c_int {
    unsafe { setgroups(groups.len() as libc::size_t, groups.as_ptr()) }
}

fn set_groups_from_str(groups: &str) {
    if !groups.is_empty() {
        let groups_vec: Vec<libc::gid_t> = groups
            .split(',')
            .map(|x| match entries::grp2gid(x) {
                Ok(g) => g,
                _ => crash!(1, "no such group: {}", x),
            })
            .collect();
        let err = set_groups(groups_vec);
        if err != 0 {
            crash!(1, "cannot set groups: {}", Error::last_os_error())
        }
    }
}

fn set_user(user: &str) {
    if !user.is_empty() {
        let user_id = entries::usr2uid(user).unwrap();
        let err = unsafe { setuid(user_id as libc::uid_t) };
        if err != 0 {
            crash!(1, "cannot set user to {}: {}", user, Error::last_os_error())
        }
    }
}
