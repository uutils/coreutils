#![crate_name = "uu_id"]

// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// Synced with:
//  http://ftp-archive.freebsd.org/mirror/FreeBSD-Archive/old-releases/i386/1.0-RELEASE/ports/shellutils/src/id.c
//  http://www.opensource.apple.com/source/shell_cmds/shell_cmds-118/id/id.c
//

#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[macro_use]
extern crate uucore;
pub use uucore::libc;
use uucore::libc::{getlogin, uid_t};
use uucore::entries::{self, Passwd, Group, Locate};
use uucore::process::{getgid, getuid, getegid, geteuid};
use std::io::Write;
use std::ffi::CStr;

macro_rules! cstr2cow {
    ($v:expr) => (
        unsafe { CStr::from_ptr($v).to_string_lossy() }
    )
}

#[cfg(not(target_os = "linux"))]
mod audit {
    pub use std::mem::uninitialized;
    use super::libc::{uid_t, pid_t, c_int, c_uint, uint64_t, dev_t};

    pub type au_id_t = uid_t;
    pub type au_asid_t = pid_t;
    pub type au_event_t = c_uint;
    pub type au_emod_t = c_uint;
    pub type au_class_t = c_int;

    #[repr(C)]
    pub struct au_mask {
        pub am_success: c_uint,
        pub am_failure: c_uint,
    }
    pub type au_mask_t = au_mask;

    #[repr(C)]
    pub struct au_tid_addr {
        pub port: dev_t,
    }
    pub type au_tid_addr_t = au_tid_addr;

    #[repr(C)]
    pub struct c_auditinfo_addr {
        pub ai_auid: au_id_t, // Audit user ID
        pub ai_mask: au_mask_t, // Audit masks.
        pub ai_termid: au_tid_addr_t, // Terminal ID.
        pub ai_asid: au_asid_t, // Audit session ID.
        pub ai_flags: uint64_t, // Audit session flags
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern "C" {
        pub fn getaudit(auditinfo_addr: *mut c_auditinfo_addr_t) -> c_int;
    }
}

static SYNTAX: &'static str = "[OPTION]... [USER]";
static SUMMARY: &'static str = "Print user and group information for the specified USER,\n or (when USER omitted) for the current user.";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = new_coreopts!(SYNTAX, SUMMARY, "");
    opts.optflag("A",
                 "",
                 "Display the process audit (not available on Linux)");
    opts.optflag("G", "", "Display the different group IDs");
    opts.optflag("g", "", "Display the effective group ID as a number");
    opts.optflag("n",
                 "",
                 "Display the name of the user or group ID for the -G, -g and -u options");
    opts.optflag("P", "", "Display the id as a password file entry");
    opts.optflag("p", "", "Make the output human-readable");
    opts.optflag("r", "", "Display the real ID for the -g and -u options");
    opts.optflag("u", "", "Display the effective user ID as a number");

    let matches = opts.parse(args);

    if matches.opt_present("A") {
        auditid();
        return 0;
    }

    let possible_pw = if matches.free.is_empty() {
        None
    } else {
        match Passwd::locate(matches.free[0].as_str()) {
            Ok(p) => Some(p),
            Err(_) => crash!(1, "No such user/group: {}", matches.free[0]),
        }
    };

    let nflag = matches.opt_present("n");
    let uflag = matches.opt_present("u");
    let gflag = matches.opt_present("g");
    let rflag = matches.opt_present("r");

    if gflag {
        let id = possible_pw.map(|p| p.gid()).unwrap_or(if rflag {
            getgid()
        } else {
            getegid()
        });
        println!("{}",
                 if nflag {
                     entries::gid2grp(id).unwrap_or(id.to_string())
                 } else {
                     id.to_string()
                 });
        return 0;
    }

    if uflag {
        let id = possible_pw.map(|p| p.uid()).unwrap_or(if rflag {
            getuid()
        } else {
            geteuid()
        });
        println!("{}",
                 if nflag {
                     entries::uid2usr(id).unwrap_or(id.to_string())
                 } else {
                     id.to_string()
                 });
        return 0;
    }

    if matches.opt_present("G") {
        println!("{}",
                 if nflag {
                     possible_pw.map(|p| p.belongs_to())
                                .unwrap_or(entries::get_groups().unwrap())
                                .iter()
                                .map(|&id| entries::gid2grp(id).unwrap())
                                .collect::<Vec<_>>()
                                .join(" ")
                 } else {
                     possible_pw.map(|p| p.belongs_to())
                                .unwrap_or(entries::get_groups().unwrap())
                                .iter()
                                .map(|&id| id.to_string())
                                .collect::<Vec<_>>()
                                .join(" ")
                 });
        return 0;
    }

    if matches.opt_present("P") {
        pline(possible_pw.map(|v| v.uid()));
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

fn pretty(possible_pw: Option<Passwd>) {
    if let Some(p) = possible_pw {
        print!("uid\t{}\ngroups\t", p.name());
        println!("{}",
                 p.belongs_to().iter().map(|&gr| entries::gid2grp(gr).unwrap()).collect::<Vec<_>>().join(" "));
    } else {
        let login = cstr2cow!(getlogin() as *const _);
        let rid = getuid();
        if let Ok(p) = Passwd::locate(rid) {
            if login == p.name() {
                println!("login\t{}", login);
            }
            println!("uid\t{}", p.name());
        } else {
            println!("uid\t{}", rid);
        }

        let eid = getegid();
        if eid == rid {
            if let Ok(p) = Passwd::locate(eid) {
                println!("euid\t{}", p.name());
            } else {
                println!("euid\t{}", eid);
            }
        }

        let rid = getgid();
        if rid != eid {
            if let Ok(g) = Group::locate(rid) {
                println!("euid\t{}", g.name());
            } else {
                println!("euid\t{}", rid);
            }
        }

        println!("groups\t{}",
                 entries::get_groups()
                     .unwrap()
                     .iter()
                     .map(|&gr| entries::gid2grp(gr).unwrap())
                     .collect::<Vec<_>>()
                     .join(" "));
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or(getuid());
    let pw = Passwd::locate(uid).unwrap();

    println!("{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
             pw.name(),
             pw.user_passwd(),
             pw.uid(),
             pw.gid(),
             pw.user_access_class(),
             pw.passwd_change_time(),
             pw.expiration(),
             pw.user_info(),
             pw.user_dir(),
             pw.user_shell());
}

#[cfg(target_os = "linux")]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or(getuid());
    let pw = Passwd::locate(uid).unwrap();

    println!("{}:{}:{}:{}:{}:{}:{}",
             pw.name(),
             pw.user_passwd(),
             pw.uid(),
             pw.gid(),
             pw.user_info(),
             pw.user_dir(),
             pw.user_shell());
}

#[cfg(target_os = "linux")]
fn auditid() {}

#[cfg(not(target_os = "linux"))]
fn auditid() {
    let mut auditinfo: audit::c_auditinfo_addr_t = unsafe { audit::uninitialized() };
    let address = &mut auditinfo as *mut audit::c_auditinfo_addr_t;
    if unsafe { audit::getaudit(address) } < 0 {
        println!("couldn't retrieve information");
        return;
    }

    println!("auid={}", auditinfo.ai_auid);
    println!("mask.success=0x{:x}", auditinfo.ai_mask.am_success);
    println!("mask.failure=0x{:x}", auditinfo.ai_mask.am_failure);
    println!("termid.port=0x{:x}", auditinfo.ai_termid.port);
    println!("asid={}", auditinfo.ai_asid);
}

fn id_print(possible_pw: Option<Passwd>, p_euid: bool, p_egid: bool) {
    let (uid, gid) = possible_pw.map(|p| (p.uid(), p.gid())).unwrap_or((getuid(), getgid()));;

    let groups = Passwd::locate(uid).unwrap().belongs_to();

    print!("uid={}({})", uid, entries::uid2usr(uid).unwrap());
    print!(" gid={}({})", gid, entries::gid2grp(gid).unwrap());

    let euid = geteuid();
    if p_euid && (euid != uid) {
        print!(" euid={}({})", euid, entries::uid2usr(euid).unwrap());
    }

    let egid = getegid();
    if p_egid && (egid != gid) {
        print!(" egid={}({})", euid, entries::gid2grp(egid).unwrap());
    }

    println!(" groups={}",
             groups.iter()
                   .map(|&gr| format!("{}({})", gr, entries::gid2grp(gr).unwrap()))
                   .collect::<Vec<_>>()
                   .join(","));
}
