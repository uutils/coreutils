#![crate_name = "chroot"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Vsevolod Velichko <torkvemada@sorokdva.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use c_types::{get_pw_from_args, get_group};
use getopts::Options;
use libc::funcs::posix88::unistd::{setgid, setuid};
use std::ffi::CString;
use std::io::{Error, Write};
use std::iter::FromIterator;
use std::path::Path;
use std::process::Command;

#[path = "../common/util.rs"] #[macro_use]mod util;
#[path = "../common/c_types.rs"]mod c_types;
#[path = "../common/filesystem.rs"]mod filesystem;

use filesystem::UUPathExt;

extern {
    fn chroot(path: *const libc::c_char) -> libc::c_int;
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
extern {
    fn setgroups(size: libc::c_int, list: *const libc::gid_t) -> libc::c_int;
}

#[cfg(target_os = "linux")]
extern {
    fn setgroups(size: libc::size_t, list: *const libc::gid_t) -> libc::c_int;
}

static NAME: &'static str = "chroot";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optopt("u",
                "user",
                "User (ID or name) to switch before running the program",
                "USER");
    opts.optopt("g", "group", "Group (ID or name) to switch to", "GROUP");
    opts.optopt("G",
                "groups",
                "Comma-separated list of groups to switch to",
                "GROUP1,GROUP2...");
    opts.optopt("",
                "userspec",
                "Colon-separated user and group to switch to. \
        Same as -u USER -g GROUP. \
        Userspec has higher preference than -u and/or -g",
                "USER:GROUP");
    opts.optflag("h", "help", "Show help");
    opts.optflag("V", "version", "Show program's version");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            help_menu(opts);
            return 1
        }
    };

    if matches.opt_present("V") {
        version();
        return 0
    }
    if matches.opt_present("h") {
        help_menu(opts);
        return 0
    }

    if matches.free.len() == 0 {
        println!("Missing operand: NEWROOT");
        println!("Try `{} --help` for more information.", NAME);
        return 1
    }

    let default_shell: &'static str = "/bin/sh";
    let default_option: &'static str = "-i";
    let user_shell = std::env::var("SHELL");

    let newroot = Path::new(&matches.free[0][..]);
    if !newroot.uu_is_dir() {
        crash!(1,
               "cannot change root directory to `{}`: no such directory",
               newroot.display());
    }

    let command: Vec<&str> = match matches.free.len() {
        1 => {
            let shell: &str = match user_shell {
                Err(_) => default_shell,
                Ok(ref s) => s.as_ref(),
            };
            vec!(shell, default_option)
        }
        _ => matches.free[1..].iter().map(|x| &x[..]).collect(),
    };

    set_context(&newroot, &matches);

    let pstatus = Command::new(command[0])
                      .args(&command[1..])
                      .status()
                      .unwrap_or_else(|e| crash!(1, "Cannot exec: {}", e));

    if pstatus.success() {
        0
    } else {
        match pstatus.code() {
            Some(i) => i,
            None => -1,
        }
    }
}

fn set_context(root: &Path, options: &getopts::Matches) {
    let userspec_str = options.opt_str("userspec");
    let user_str = options.opt_str("user").unwrap_or_default();
    let group_str = options.opt_str("group").unwrap_or_default();
    let groups_str = options.opt_str("groups").unwrap_or_default();
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
        chroot(CString::new(".".as_bytes()).unwrap().as_bytes_with_nul().as_ptr() as *const libc::c_char)
    };
    if err != 0 {
        crash!(1,
               "cannot chroot to {}: {}",
               root_str,
               Error::last_os_error())
    };
}

fn set_main_group(group: &str) {
    if !group.is_empty() {
        let group_id = match get_group(group) {
            None => crash!(1, "no such group: {}", group),
            Some(g) => g.gr_gid,
        };
        let err = unsafe { setgid(group_id) };
        if err != 0 {
            crash!(1,
                   "cannot set gid to {}: {}",
                   group_id,
                   Error::last_os_error())
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
        let groups_vec: Vec<libc::gid_t> = FromIterator::from_iter(groups.split(',').map(
                |x| match get_group(x) {
                    None => crash!(1, "no such group: {}", x),
                    Some(g) => g.gr_gid
                }));
        let err = set_groups(groups_vec);
        if err != 0 {
            crash!(1, "cannot set groups: {}", Error::last_os_error())
        }
    }
}

fn set_user(user: &str) {
    if !user.is_empty() {
        let user_id = get_pw_from_args(&vec!(user.to_string())).unwrap().pw_uid;
        let err = unsafe { setuid(user_id as libc::uid_t) };
        if err != 0 {
            crash!(1,
                   "cannot set user to {}: {}",
                   user,
                   Error::last_os_error())
        }
    }
}

fn version() {
    println!("{} {}", NAME, VERSION)
}

fn help_menu(options: Options) {
    let msg = format!("{0} {1}

Usage:
  {0} [OPTION]... NEWROOT [COMMAND [ARG]...]

Run COMMAND with root directory set to NEWROOT.
If COMMAND is not specified, it defaults to '$(SHELL) -i'.
If $(SHELL) is not set, /bin/sh is used.",
                      NAME,
                      VERSION);

    print!("{}", options.usage(&msg));
}
