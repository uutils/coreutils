#![crate_id(name="id", version="1.0.0", author="Alan Andrade")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with:
 *  http://ftp-archive.freebsd.org/mirror/FreeBSD-Archive/old-releases/i386/1.0-RELEASE/ports/shellutils/src/id.c
 *  http://www.opensource.apple.com/source/shell_cmds/shell_cmds-118/id/id.c
 */

#![allow(non_camel_case_types)]
#![feature(macro_rules)]
extern crate getopts;
extern crate libc;

use std::os;
use std::ptr::read;
use libc::{
    c_int,
    uid_t,
    getgid,
    getuid
};
use libc::funcs::posix88::unistd::{getegid, geteuid, getlogin};
use std::str::raw::from_c_str;
use getopts::{getopts, optflag, usage};
use c_types::{
    c_passwd,
    c_group,
    get_groups,
    get_group_list,
    get_pw_from_args,
    getpwuid,
    group
};

#[path = "../common/util.rs"] mod util;
#[path = "../common/c_types.rs"] mod c_types;

#[cfg(not(target_os = "linux"))]
mod audit {
    pub use std::mem::uninitialized;
    use libc::{uid_t, pid_t, c_int, c_uint, uint64_t, dev_t};

    pub type au_id_t    = uid_t;
    pub type au_asid_t  = pid_t;
    pub type au_event_t = c_uint;
    pub type au_emod_t  = c_uint;
    pub type au_class_t = c_int;

    pub struct au_mask {
        pub am_success: c_uint,
        pub am_failure: c_uint
    }
    pub type au_mask_t = au_mask;

    pub struct au_tid_addr {
        pub port: dev_t,
    }
    pub type au_tid_addr_t = au_tid_addr;

    pub struct c_auditinfo_addr {
        pub ai_auid: au_id_t,           /* Audit user ID */
        pub ai_mask: au_mask_t,         /* Audit masks. */
        pub ai_termid: au_tid_addr_t,   /* Terminal ID. */
        pub ai_asid: au_asid_t,         /* Audit session ID. */
        pub ai_flags: uint64_t          /* Audit session flags */
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern {
        pub fn getaudit(auditinfo_addr: *c_auditinfo_addr_t) -> c_int;
    }
}

extern {
    fn getgrgid(gid: uid_t) -> *c_group;
}

static NAME: &'static str = "id";

#[allow(dead_code)]
fn main () { os::set_exit_status(uumain(os::args())); }

pub fn uumain(args: Vec<String>) -> int {
    let args_t = args.tail();

    let options = [
        optflag("h", "", "Show help"),
        optflag("A", "", "Display the process audit (not available on Linux)"),
        optflag("G", "", "Display the different group IDs"),
        optflag("g", "", "Display the effective group ID as a number"),
        optflag("n", "", "Display the name of the user or group ID for the -G, -g and -u options"),
        optflag("P", "", "Display the id as a password file entry"),
        optflag("p", "", "Make the output human-readable"),
        optflag("r", "", "Display the real ID for the -g and -u options"),
        optflag("u", "", "Display the effective user ID as a number")
    ];

    let matches = match getopts(args_t, options) {
        Ok(m) => { m },
        Err(_) => {
            println!("{:s}", usage(NAME, options));
            return 0;
        }
    };

    if matches.opt_present("h") {
        println!("{:s}", usage(NAME, options));
        return 0;
    }

    if matches.opt_present("A") {
        auditid();
        return 0;
    }


    let possible_pw = get_pw_from_args(&matches.free);

    let nflag = matches.opt_present("n");
    let uflag = matches.opt_present("u");
    let gflag = matches.opt_present("g");
    let rflag = matches.opt_present("r");

    if gflag {
        let id = if possible_pw.is_some() {
            possible_pw.unwrap().pw_gid
        } else {
            if rflag {
                unsafe { getgid() }
            } else {
                unsafe { getegid() }
            }
        };
        let gr = unsafe { getgrgid(id) };

        if nflag && gr.is_not_null() {
            let gr_name = unsafe { from_c_str(read(gr).gr_name) };
            println!("{:s}", gr_name);
        } else {
            println!("{:u}", id);
        }
        return 0;
    }

    if uflag {
        let id = if possible_pw.is_some() {
            possible_pw.unwrap().pw_uid
        } else if rflag {
            unsafe { getgid() }
        } else {
            unsafe { getegid() }
        };

        let pw = unsafe { getpwuid(id) };
        if nflag && pw.is_not_null() {
            let pw_name = unsafe {
                from_c_str(read(pw).pw_name)
            };
            println!("{:s}", pw_name);
        } else {
            println!("{:u}", id);
        }

        return 0;
    }

    if matches.opt_present("G") {
        group(possible_pw, nflag);
        return 0;
    }

    if matches.opt_present("P") {
        pline(possible_pw);
        return 0;
    };

    if matches.opt_present("p") {
        pretty(possible_pw);
        return 0;
    }

    if possible_pw.is_some() {
        id_print(possible_pw, false, false)
    } else {
        id_print(possible_pw, true, true)
    }

    0
}

fn pretty(possible_pw: Option<c_passwd>) {
    if possible_pw.is_some() {
        let pw = possible_pw.unwrap();

        let pw_name = unsafe { from_c_str(pw.pw_name) };
        print!("uid\t{:s}\ngroups\t", pw_name);
        group(possible_pw, true);
    } else {
        let login = unsafe { from_c_str(getlogin()) };
        let rid = unsafe { getuid() };
        let pw = unsafe { getpwuid(rid) };

        let is_same_user = unsafe {
            from_c_str(read(pw).pw_name) == login
        };

        if pw.is_null() || is_same_user {
            println!("login\t{:s}", login);
        }

        if pw.is_not_null() {
            println!(
                "uid\t{:s}",
                unsafe { from_c_str(read(pw).pw_name) })
        } else {
            println!("uid\t{:u}\n", rid);
        }

        let eid = unsafe { getegid() };
        if eid == rid {
            let pw = unsafe { getpwuid(eid) };
            if pw.is_not_null() {
                println!(
                    "euid\t{:s}",
                    unsafe { from_c_str(read(pw).pw_name) });
            } else {
                println!("euid\t{:u}", eid);
            }
        }

        let rid = unsafe { getgid() };

        if rid != eid {
            let gr = unsafe { getgrgid(rid) };
            if gr.is_not_null() {
                println!(
                    "rgid\t{:s}",
                    unsafe { from_c_str(read(gr).gr_name) });
            } else {
                println!("rgid\t{:u}", rid);
            }
        }

        print!("groups\t");
        group(None, true);
    }
}

#[cfg(target_os = "macos")]
fn pline(possible_pw: Option<c_passwd>) {
    let pw = if possible_pw.is_none() {
        unsafe { read(getpwuid(getuid())) }
    } else {
        possible_pw.unwrap()
    };

    let pw_name     = unsafe { from_c_str(pw.pw_name)  };
    let pw_passwd   = unsafe { from_c_str(pw.pw_passwd)};
    let pw_class    = unsafe { from_c_str(pw.pw_class) };
    let pw_gecos    = unsafe { from_c_str(pw.pw_gecos) };
    let pw_dir      = unsafe { from_c_str(pw.pw_dir)   };
    let pw_shell    = unsafe { from_c_str(pw.pw_shell) };

    println!(
        "{:s}:{:s}:{:u}:{:u}:{:s}:{:d}:{:d}:{:s}:{:s}:{:s}",
        pw_name,
        pw_passwd,
        pw.pw_uid,
        pw.pw_gid,
        pw_class,
        pw.pw_change,
        pw.pw_expire,
        pw_gecos,
        pw_dir,
        pw_shell);
}

#[cfg(target_os = "linux")]
fn pline(possible_pw: Option<c_passwd>) {
    let pw = if possible_pw.is_none() {
        unsafe { read(getpwuid(getuid())) }
    } else {
        possible_pw.unwrap()
    };

    let pw_name     = unsafe { from_c_str(pw.pw_name)  };
    let pw_passwd   = unsafe { from_c_str(pw.pw_passwd)};
    let pw_gecos    = unsafe { from_c_str(pw.pw_gecos) };
    let pw_dir      = unsafe { from_c_str(pw.pw_dir)   };
    let pw_shell    = unsafe { from_c_str(pw.pw_shell) };

    println!(
        "{:s}:{:s}:{:u}:{:u}:{:s}:{:s}:{:s}",
        pw_name,
        pw_passwd,
        pw.pw_uid,
        pw.pw_gid,
        pw_gecos,
        pw_dir,
        pw_shell);
}

#[cfg(target_os = "linux")]
fn auditid() { }

#[cfg(not(target_os = "linux"))]
fn auditid() {
    let auditinfo: audit::c_auditinfo_addr_t = unsafe { audit::uninitialized() };
    let address = &auditinfo as *audit::c_auditinfo_addr_t;
    if  unsafe { audit::getaudit(address) } < 0 {
        println!("Couldlnt retrieve information");
        return;
    }

    println!("auid={:u}", auditinfo.ai_auid);
    println!("mask.success=0x{:x}", auditinfo.ai_mask.am_success);
    println!("mask.failure=0x{:x}", auditinfo.ai_mask.am_failure);
    println!("termid.port=0x{:x}", auditinfo.ai_termid.port);
    println!("asid={:d}", auditinfo.ai_asid);
}

fn id_print(possible_pw: Option<c_passwd>,
            p_euid: bool,
            p_egid: bool) {

    let uid;
    let gid;

    if possible_pw.is_some() {
        uid = possible_pw.unwrap().pw_uid;
        gid = possible_pw.unwrap().pw_gid;
    } else {
        uid = unsafe { getuid() };
        gid = unsafe { getgid() };
    }

    let groups = match possible_pw {
        Some(pw) => get_group_list(pw.pw_name, pw.pw_gid),
        None => get_groups(),
    };

    let groups = groups.unwrap_or_else(|errno| {
        crash!(1, "failed to get group list (errno={:d})", errno);
    });

    if possible_pw.is_some() {
        print!(
            "uid={:u}({:s})",
            uid,
            unsafe { from_c_str(possible_pw.unwrap().pw_name) });
    } else {
        print!("uid={:u}", unsafe { getuid() });
    }

    print!(" gid={:u}", gid);
    let gr = unsafe { getgrgid(gid) };
    if gr.is_not_null() {
        print!(
            "({:s})",
            unsafe { from_c_str(read(gr).gr_name) });
    }

    let euid = unsafe { geteuid() };
    if p_euid && (euid != uid) {
        print!(" euid={:u}", euid);
        let pw = unsafe { getpwuid(euid) };
        if pw.is_not_null() {
            print!(
                "({:s})",
                unsafe { from_c_str(read(pw).pw_name) });
        }
    }

    let egid = unsafe { getegid() };
    if p_egid && (egid != gid) {
        print!(" egid={:u}", egid);
        unsafe {
            let grp = getgrgid(egid);
            if grp.is_not_null() {
                print!("({:s})", from_c_str(read(grp).gr_name));
            }
        }
    }

    if groups.len() > 0 {
        print!(" groups=");

        let mut first = true;
        for &gr in groups.iter() {
            if !first { print!(",") }
            print!("{:u}", gr);
            let group = unsafe { getgrgid(gr) };
            if group.is_not_null() {
                let name = unsafe {
                    from_c_str(read(group).gr_name)
                };
                print!("({:s})", name);
            }
            first = false
        }
    }

    println!("");
}
