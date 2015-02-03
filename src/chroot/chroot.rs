#![crate_name = "chroot"]
#![feature(collections, core, io, libc, os, path, rustc_private, std_misc)]

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

use getopts::{optflag, optopt, getopts, usage};
use c_types::{get_pw_from_args, get_group};
use libc::funcs::posix88::unistd::{execvp, setuid, setgid};
use std::ffi::{c_str_to_bytes, CString};
use std::old_io::fs::PathExtensions;
use std::iter::FromIterator;

#[path = "../common/util.rs"] #[macro_use] mod util;
#[path = "../common/c_types.rs"] mod c_types;

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

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0];

    let options = [
        optopt("u", "user", "User (ID or name) to switch before running the program", "USER"),
        optopt("g", "group", "Group (ID or name) to switch to", "GROUP"),
        optopt("G", "groups", "Comma-separated list of groups to switch to", "GROUP1,GROUP2…"),
        optopt("", "userspec", "Colon-separated user and group to switch to. \
                                Same as -u USER -g GROUP. \
                                Userspec has higher preference than -u and/or -g", "USER:GROUP"),
        optflag("h", "help", "Show help"),
        optflag("V", "version", "Show program's version")
    ];

    let opts = match getopts(args.tail(), &options) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            help_menu(program.as_slice(), &options);
            return 1
        }
    };

    if opts.opt_present("V") { version(); return 0 }
    if opts.opt_present("h") { help_menu(program.as_slice(), &options); return 0 }

    if opts.free.len() == 0 {
        println!("Missing operand: NEWROOT");
        println!("Try `{} --help` for more information.", program.as_slice());
        return 1
    }

    let default_shell: &'static str = "/bin/sh";
    let default_option: &'static str = "-i";
    let user_shell = std::os::getenv("SHELL");

    let newroot = Path::new(opts.free[0].as_slice());
    if !newroot.is_dir() {
        crash!(1, "cannot change root directory to `{}`: no such directory", newroot.display());
    }

    let command: Vec<&str> = match opts.free.len() {
        1 => {
            let shell: &str = match user_shell {
                None => default_shell,
                Some(ref s) => s.as_slice()
            };
            vec!(shell, default_option)
        }
        _ => opts.free[1..opts.free.len()].iter().map(|x| x.as_slice()).collect()
    };

    set_context(&newroot, &opts);

    unsafe {
        let executable = CString::from_slice(command[0].as_bytes()).as_slice_with_nul().as_ptr();
        let mut command_parts: Vec<*const i8> = command.iter().map(|x| CString::from_slice(x.as_bytes()).as_slice_with_nul().as_ptr()).collect();
        command_parts.push(std::ptr::null());
        execvp(executable as *const libc::c_char, command_parts.as_ptr() as *mut *const libc::c_char) as isize
    }
}

fn set_context(root: &Path, options: &getopts::Matches) {
    let userspec_str = options.opt_str("userspec");
    let user_str = options.opt_str("user").unwrap_or_default();
    let group_str = options.opt_str("group").unwrap_or_default();
    let groups_str = options.opt_str("groups").unwrap_or_default();
    let userspec = match userspec_str {
        Some(ref u) => {
            let s: Vec<&str> = u.as_slice().split(':').collect();
            if s.len() != 2 {
                crash!(1, "invalid userspec: `{}`", u.as_slice())
            };
            s
        }
        None => Vec::new()
    };
    let user = if userspec.is_empty() { user_str.as_slice() } else { userspec[0].as_slice() };
    let group = if userspec.is_empty() { group_str.as_slice() } else { userspec[1].as_slice() };

    enter_chroot(root);

    set_groups_from_str(groups_str.as_slice());
    set_main_group(group);
    set_user(user);
}

fn enter_chroot(root: &Path) {
    let root_str = root.display();
    std::os::change_dir(root).unwrap();
    let err = unsafe {
        chroot(CString::from_slice(b".").as_slice_with_nul().as_ptr() as *const libc::c_char)
    };
    if err != 0 {
        crash!(1, "cannot chroot to {}: {}", root_str, strerror(err).as_slice())
    };
}

fn set_main_group(group: &str) {
    if !group.is_empty() {
        let group_id = match get_group(group) {
            None => crash!(1, "no such group: {}", group),
            Some(g) => g.gr_gid
        };
        let err = unsafe { setgid(group_id) };
        if err != 0 {
            crash!(1, "cannot set gid to {}: {}", group_id, strerror(err).as_slice())
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn set_groups(groups: Vec<libc::gid_t>) -> libc::c_int {
    unsafe {
        setgroups(groups.len() as libc::c_int,
                  groups.as_slice().as_ptr())
    }
}

#[cfg(target_os = "linux")]
fn set_groups(groups: Vec<libc::gid_t>) -> libc::c_int {
    unsafe {
        setgroups(groups.len() as libc::size_t,
                  groups.as_slice().as_ptr())
    }
}

fn set_groups_from_str(groups: &str) {
    if !groups.is_empty() {
        let groups_vec: Vec<libc::gid_t> = FromIterator::from_iter(
            groups.split(',').map(
                |x| match get_group(x) {
                    None => crash!(1, "no such group: {}", x),
                    Some(g) => g.gr_gid
                })
            );
        let err = set_groups(groups_vec);
        if err != 0 {
            crash!(1, "cannot set groups: {}", strerror(err).as_slice())
        }
    }
}

fn set_user(user: &str) {
    if !user.is_empty() {
        let user_id = get_pw_from_args(&vec!(String::from_str(user))).unwrap().pw_uid;
        let err = unsafe { setuid(user_id as libc::uid_t) };
        if err != 0 {
            crash!(1, "cannot set user to {}: {}", user, strerror(err).as_slice())
        }
    }
}

fn strerror(errno: i32) -> String {
    unsafe {
        let err = libc::funcs::c95::string::strerror(errno) as *const libc::c_char;
        let bytes= c_str_to_bytes(&err);
        String::from_utf8_lossy(bytes).to_string()
    }
}

fn version() {
    println!("{} v{}", NAME, VERSION)
}

fn help_menu(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("Usage:");
    println!("  {} [OPTION]… NEWROOT [COMMAND [ARG]…]", program);
    println!("");
    print!("{}", usage(
            "Run COMMAND with root directory set to NEWROOT.\n\
             If COMMAND is not specified, it defaults to '${SHELL} -i'. \
             If ${SHELL} is not set, /bin/sh is used.", options))
}
