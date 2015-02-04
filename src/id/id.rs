#![crate_name = "id"]
#![feature(collections, core, libc, rustc_private, std_misc)]

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
extern crate getopts;
extern crate libc;

use std::ffi::c_str_to_bytes;
use std::ptr::read;
use libc::{
    uid_t,
    getgid,
    getuid
};
use libc::funcs::posix88::unistd::{getegid, geteuid, getlogin};
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

#[path = "../common/util.rs"] #[macro_use] mod util;
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

    #[repr(C)]
    pub struct au_mask {
        pub am_success: c_uint,
        pub am_failure: c_uint
    }
    pub type au_mask_t = au_mask;

    #[repr(C)]
    pub struct au_tid_addr {
        pub port: dev_t,
    }
    pub type au_tid_addr_t = au_tid_addr;

    #[repr(C)]
    pub struct c_auditinfo_addr {
        pub ai_auid: au_id_t,           /* Audit user ID */
        pub ai_mask: au_mask_t,         /* Audit masks. */
        pub ai_termid: au_tid_addr_t,   /* Terminal ID. */
        pub ai_asid: au_asid_t,         /* Audit session ID. */
        pub ai_flags: uint64_t          /* Audit session flags */
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern {
        pub fn getaudit(auditinfo_addr: *mut c_auditinfo_addr_t) -> c_int;
    }
}

extern {
    fn getgrgid(gid: uid_t) -> *const c_group;
}

static NAME: &'static str = "id";

pub fn uumain(args: Vec<String>) -> isize {
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

    let matches = match getopts(args_t, &options) {
        Ok(m) => { m },
        Err(_) => {
            println!("{}", usage(NAME, &options));
            return 1;
        }
    };

    if matches.opt_present("h") {
        println!("{}", usage(NAME, &options));
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

        if nflag && !gr.is_null() {
            let gr_name = unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(gr).gr_name)).to_string() };
            println!("{}", gr_name);
        } else {
            println!("{}", id);
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
        if nflag && !pw.is_null() {
            let pw_name = unsafe {
                String::from_utf8_lossy(c_str_to_bytes(&read(pw).pw_name)).to_string()
            };
            println!("{}", pw_name);
        } else {
            println!("{}", id);
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

        let pw_name = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_name)).to_string() };
        print!("uid\t{}\ngroups\t", pw_name);
        group(possible_pw, true);
    } else {
        let login = unsafe { String::from_utf8_lossy(c_str_to_bytes(&(getlogin() as *const i8))).to_string() };
        let rid = unsafe { getuid() };
        let pw = unsafe { getpwuid(rid) };

        let is_same_user = unsafe {
            String::from_utf8_lossy(c_str_to_bytes(&read(pw).pw_name)).to_string() == login
        };

        if pw.is_null() || is_same_user {
            println!("login\t{}", login);
        }

        if !pw.is_null() {
            println!(
                "uid\t{}",
                unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(pw).pw_name)).to_string() })
        } else {
            println!("uid\t{}\n", rid);
        }

        let eid = unsafe { getegid() };
        if eid == rid {
            let pw = unsafe { getpwuid(eid) };
            if !pw.is_null() {
                println!(
                    "euid\t{}",
                    unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(pw).pw_name)).to_string() });
            } else {
                println!("euid\t{}", eid);
            }
        }

        let rid = unsafe { getgid() };

        if rid != eid {
            let gr = unsafe { getgrgid(rid) };
            if !gr.is_null() {
                println!(
                    "rgid\t{}",
                    unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(gr).gr_name)).to_string() });
            } else {
                println!("rgid\t{}", rid);
            }
        }

        print!("groups\t");
        group(None, true);
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn pline(possible_pw: Option<c_passwd>) {
    let pw = if possible_pw.is_none() {
        unsafe { read(getpwuid(getuid())) }
    } else {
        possible_pw.unwrap()
    };

    let pw_name     = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_name  )).to_string()};
    let pw_passwd   = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_passwd)).to_string()};
    let pw_class    = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_class )).to_string()};
    let pw_gecos    = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_gecos )).to_string()};
    let pw_dir      = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_dir   )).to_string()};
    let pw_shell    = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_shell )).to_string()};

    println!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
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

    let pw_name     = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_name  )).to_string()};
    let pw_passwd   = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_passwd)).to_string()};
    let pw_gecos    = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_gecos )).to_string()};
    let pw_dir      = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_dir   )).to_string()};
    let pw_shell    = unsafe { String::from_utf8_lossy(c_str_to_bytes(&pw.pw_shell )).to_string()};

    println!(
        "{}:{}:{}:{}:{}:{}:{}",
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
    let mut auditinfo: audit::c_auditinfo_addr_t = unsafe { audit::uninitialized() };
    let address = &mut auditinfo as *mut audit::c_auditinfo_addr_t;
    if  unsafe { audit::getaudit(address) } < 0 {
        println!("couldn't retrieve information");
        return;
    }

    println!("auid={}", auditinfo.ai_auid);
    println!("mask.success=0x{:x}", auditinfo.ai_mask.am_success);
    println!("mask.failure=0x{:x}", auditinfo.ai_mask.am_failure);
    println!("termid.port=0x{:x}", auditinfo.ai_termid.port);
    println!("asid={}", auditinfo.ai_asid);
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
        Some(pw) => Ok(get_group_list(pw.pw_name, pw.pw_gid)),
        None => get_groups(),
    };

    let groups = groups.unwrap_or_else(|errno| {
        crash!(1, "failed to get group list (errno={})", errno);
    });

    if possible_pw.is_some() {
        print!(
            "uid={}({})",
            uid,
            unsafe { String::from_utf8_lossy(c_str_to_bytes(&possible_pw.unwrap().pw_name)).to_string() });
    } else {
        print!("uid={}", unsafe { getuid() });
    }

    print!(" gid={}", gid);
    let gr = unsafe { getgrgid(gid) };
    if !gr.is_null() {
        print!(
            "({})",
            unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(gr).gr_name)).to_string() });
    }

    let euid = unsafe { geteuid() };
    if p_euid && (euid != uid) {
        print!(" euid={}", euid);
        let pw = unsafe { getpwuid(euid) };
        if !pw.is_null() {
            print!(
                "({})",
                unsafe { String::from_utf8_lossy(c_str_to_bytes(&read(pw).pw_name)).to_string() });
        }
    }

    let egid = unsafe { getegid() };
    if p_egid && (egid != gid) {
        print!(" egid={}", egid);
        unsafe {
            let grp = getgrgid(egid);
            if !grp.is_null() {
                print!("({})", String::from_utf8_lossy(c_str_to_bytes(&read(grp).gr_name)).to_string());
            }
        }
    }

    if groups.len() > 0 {
        print!(" groups=");

        let mut first = true;
        for &gr in groups.iter() {
            if !first { print!(",") }
            print!("{}", gr);
            let group = unsafe { getgrgid(gr) };
            if !group.is_null() {
                let name = unsafe {
                    String::from_utf8_lossy(c_str_to_bytes(&read(group).gr_name)).to_string()
                };
                print!("({})", name);
            }
            first = false
        }
    }

    println!("");
}
