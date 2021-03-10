// This file is part of the uutils coreutils package.
//
// (c) Vsevolod Velichko <torkvemada@sorokdva.net>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) NEWROOT Userspec pstatus

#[macro_use]
extern crate uucore;
use clap::{App, Arg};
use std::ffi::CString;
use std::io::Error;
use std::path::Path;
use std::process::Command;
use uucore::entries;
use uucore::libc::{self, chroot, setgid, setgroups, setuid};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static NAME: &str = "chroot";
static ABOUT: &str = "Run COMMAND with root directory set to NEWROOT.";
static SYNTAX: &str = "[OPTION]... NEWROOT [COMMAND [ARG]...]";
// static SUMMARY: &str = "Run COMMAND with root directory set to NEWROOT.";
// static LONG_HELP: &str = "
//  If COMMAND is not specified, it defaults to '$(SHELL) -i'.
//  If $(SHELL) is not set, /bin/sh is used.
// ";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(SYNTAX)
        // .help(SUMMARY)
        // .after_help(LONG_HELP)
        .arg(Arg::with_name("newroot").hidden(true))
        .arg(
            Arg::with_name("user")
                .short("u")
                .long("user")
                .help("User (ID or name) to switch before running the program")
                .value_name("USER"),
        )
        .arg(
            Arg::with_name("group")
                .short("g")
                .long("group")
                .help("Group (ID or name) to switch to")
                .value_name("GROUP"),
        )
        .arg(
            Arg::with_name("groups")
                .short("G")
                .long("groups")
                .help("Comma-separated list of groups to switch to")
                .value_name("GROUP1,GROUP2..."),
        )
        .arg(
            Arg::with_name("userspec")
                .long("userspec")
                .help(
                    "Colon-separated user and group to switch to. \
             Same as -u USER -g GROUP. \
             Userspec has higher preference than -u and/or -g",
                )
                .value_name("USER:GROUP"),
        )
        .get_matches_from(args);

    if matches.args.is_empty() {
        println!("Missing operand: NEWROOT");
        println!("Try `{} --help` for more information.", NAME);
        return 1;
    }

    let default_shell: &'static str = "/bin/sh";
    let default_option: &'static str = "-i";
    let user_shell = std::env::var("SHELL");
    println!("{:?}", matches);

    let newroot: &Path = match matches.value_of("newroot") {
        Some(v) => Path::new(v),
        None => crash!(1, "chroot: missing operand"),
    };

    if !newroot.is_dir() {
        crash!(
            1,
            "cannot change root directory to `{}`: no such directory",
            newroot.display()
        );
    }

    let command: Vec<&str> = match matches.args.len() {
        1 => {
            let shell: &str = match user_shell {
                Err(_) => default_shell,
                Ok(ref s) => s.as_ref(),
            };
            vec![shell, default_option]
        }
        _ => {
            let mut vector: Vec<&str> = Vec::new();
            for (&k, v) in matches.args.iter() {
                vector.push(k.clone());
                vector.push(&v.vals[0].to_str().unwrap());
            }
            vector
        }
    };

    set_context(&newroot, &matches);

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
    let userspec_str = options.value_of("userspec");
    let user_str = options.value_of("user").unwrap_or_default();
    let group_str = options.value_of("group").unwrap_or_default();
    let groups_str = options.value_of("groups").unwrap_or_default();
    let userspec = match userspec_str {
        Some(ref u) => {
            let s: Vec<&str> = u.split(':').collect();
            if s.len() != 2 {
                crash!(1, "invalid userspec: `{}`", u)
            };
            s
        }
        None => Vec::new(),
    };
    let user = if userspec.is_empty() {
        &user_str[..]
    } else {
        &userspec[0][..]
    };
    let group = if userspec.is_empty() {
        &group_str[..]
    } else {
        &userspec[1][..]
    };

    enter_chroot(root);

    set_groups_from_str(&groups_str[..]);
    set_main_group(&group[..]);
    set_user(&user[..]);
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

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
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
