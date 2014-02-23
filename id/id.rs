#[crate_id(name="id", version="1.0.0", author="Alan Andrade")];

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

#[allow(non_camel_case_types)];

extern crate getopts;

use std::{libc, os, vec};
use std::ptr::read;
use std::libc::{c_char, c_int, time_t, uid_t, getgid, getegid, getuid, getlogin};
use std::str::raw::from_c_str;
use getopts::{getopts, optflag, usage};

// These could be extracted into their own file
struct c_passwd {
    pw_name:    *c_char,    /* user name */
    pw_passwd:  *c_char,    /* user name */
    pw_uid:     c_int,      /* user uid */
    pw_gid:     c_int,      /* user gid */
    pw_change:  time_t,
    pw_class:   *c_char,
    pw_gecos:   *c_char,
    pw_dir:     *c_char,
    pw_shell:   *c_char,
    pw_expire:  time_t
}

struct c_group {
    gr_name: *c_char /* group name */
}

#[cfg(not(target_os = "linux"))]
mod audit {
    pub use std::mem::uninit;
    use std::libc::{uid_t, pid_t, c_int, c_uint, uint64_t, dev_t};

    pub type au_id_t    = uid_t;
    pub type au_asid_t  = pid_t;
    pub type au_event_t = c_uint;
    pub type au_emod_t  = c_uint;
    pub type au_class_t = c_int;

    pub struct au_mask {
        am_success: c_uint,
        am_failure: c_uint
    }
    pub type au_mask_t = au_mask;

    pub struct au_tid_addr {
        port: dev_t,
    }
    pub type au_tid_addr_t = au_tid_addr;

    pub struct c_auditinfo_addr {
        ai_auid: au_id_t,           /* Audit user ID */
        ai_mask: au_mask_t,         /* Audit masks. */
        ai_termid: au_tid_addr_t,   /* Terminal ID. */
        ai_asid: au_asid_t,         /* Audit session ID. */
        ai_flags: uint64_t          /* Audit session flags */
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern {
        pub fn getaudit(auditinfo_addr: *c_auditinfo_addr_t) -> c_int;
    }
}

extern {
    fn getpwuid(uid: uid_t) -> *c_passwd;
    fn getgrgid(gid: uid_t) -> *c_group;
    fn getpwnam(login: *c_char) -> *c_passwd;
    fn getgrouplist(name:   *c_char,
                    basegid: c_int,
                    groups: *c_int,
                    ngroups: *mut c_int) -> c_int;
}

static PROGRAM: &'static str = "id";

fn main () {
    let args = os::args();
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
            println!("{:s}", usage(PROGRAM, options));
            return;
        }
    };

    if matches.opt_present("h") {
        println!("{:s}", usage(PROGRAM, options));
        return;
    }

    if matches.opt_present("A") {
        auditid();
        return;
    }


    let possible_pw = if matches.free.len() == 1 {
        let username = matches.free[0].clone();

        // Passed user by id
        if username.chars().all(|c| c.is_digit()) {
            let id = from_str::<u32>(username).unwrap();
            let pw_pointer = unsafe { getpwuid(id) };

            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                no_such_user(username);
                return;
            }

        // Passed the username as a string
        } else {
            let pw_pointer = unsafe {
                getpwnam(username.as_slice().as_ptr() as *i8)
            };
            if pw_pointer.is_not_null() {
                Some(unsafe { read(pw_pointer) })
            } else {
                no_such_user(username);
                return;
            }
        }
    } else {
        None
    };


    let nflag = matches.opt_present("n");
    let uflag = matches.opt_present("u");
    let gflag = matches.opt_present("g");
    let rflag = matches.opt_present("r");

    if gflag {
        let id = if possible_pw.is_some() {
            possible_pw.unwrap().pw_gid
        } else {
            if rflag {
                unsafe { getgid() as i32 }
            } else {
                unsafe { getegid() as i32 }
            }
        } as u32;
        let gr = unsafe { getgrgid(id) };

        if nflag && gr.is_not_null() {
            let gr_name = unsafe { from_c_str(read(gr).gr_name) };
            println!("{:s}", gr_name);
        } else {
            println!("{:u}", id);
        }
        return;
    }

    if uflag {
        let id = if possible_pw.is_some() {
            possible_pw.unwrap().pw_uid
        } else if rflag {
            unsafe { getgid() as i32 }
        } else {
            unsafe { getegid() as i32 }
        };

        let pw = unsafe { getpwuid(id as u32) };
        if nflag && pw.is_not_null() {
            let pw_name = unsafe {
                from_c_str(read(pw).pw_name)
            };
            println!("{:s}", pw_name);
        } else {
            println!("{:d}", id);
        }

        return;
    }

    if matches.opt_present("G") {
        group(possible_pw, nflag);
        return;
    }

    if matches.opt_present("P") {
        pline(possible_pw);
        return;
    };

    if matches.opt_present("p") {
        pretty(possible_pw);
        return;
    }

    if possible_pw.is_some() {
        id_print(possible_pw, true, false, false)
    } else {
        id_print(possible_pw, false, true, true)
    }
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
        "{:s}:{:s}:{:d}:{:d}:{:s}:{:d}:{:d}:{:s}:{:s}:{:s}",
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

fn no_such_user(username: ~str) {
    println!("{:s}: {:s}: no such user", PROGRAM, username.as_slice());
}

static NGROUPS: i32 = 20;

fn group(possible_pw: Option<c_passwd>, nflag: bool) {
    let mut groups = vec::with_capacity(NGROUPS as uint);
    let mut ngroups;

    if possible_pw.is_some() {
        ngroups = NGROUPS;
        unsafe {
            getgrouplist(
                possible_pw.unwrap().pw_name,
                possible_pw.unwrap().pw_gid,
                groups.as_ptr(),
                &mut ngroups);
        }
    } else {
        ngroups = unsafe {
            libc::getgroups(NGROUPS, groups.as_mut_ptr() as *mut u32)
        };
    }


    unsafe { groups.set_len(ngroups as uint) };

    for &g in groups.iter() {
        if nflag {
            let group = unsafe { getgrgid(g as u32) };
            if group.is_not_null() {
                let name = unsafe {
                    from_c_str(read(group).gr_name)
                };
                print!("{:s} ", name);
            }
        } else {
            print!("{:d} ", g);
        }
    }

    println!("");
}

#[cfg(target_os = "linux")]
fn auditid() { }

#[cfg(not(target_os = "linux"))]
fn auditid() {
    let auditinfo: audit::c_auditinfo_addr_t = unsafe { audit::uninit() };
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
            use_ggl: bool,
            p_euid: bool,
            p_egid: bool) {

    let uid;
    let gid;

    if possible_pw.is_some() {
        uid = possible_pw.unwrap().pw_uid;
        gid = possible_pw.unwrap().pw_gid;
    } else {
        uid = unsafe { getuid() as i32 };
        gid = unsafe { getgid() as i32 };
    }

    let mut ngroups;
    let mut groups = vec::with_capacity(NGROUPS as uint);

    if use_ggl && possible_pw.is_some() {
        ngroups = NGROUPS;
        let pw_name = possible_pw.unwrap().pw_name;

        unsafe { getgrouplist(pw_name, gid, groups.as_ptr(), &mut ngroups) };
    } else {
        ngroups = unsafe {
            libc::getgroups(NGROUPS, groups.as_mut_ptr() as *mut u32)
        };
    }

    if possible_pw.is_some() {
        print!(
            "uid={:d}({:s})",
            uid,
            unsafe { from_c_str(possible_pw.unwrap().pw_name) });
    } else {
        print!("uid={:u}", unsafe { getuid() });
    }

    print!(" gid={:d}", gid);
    let gr = unsafe { getgrgid(gid as u32) };
    if gr.is_not_null() {
        print!(
            "({:s})",
            unsafe { from_c_str(read(gr).gr_name) });
    }

    let euid = unsafe { libc::geteuid() };
    if p_euid && (euid != uid as u32) {
        print!(" euid={:u}", euid);
        let pw = unsafe { getpwuid(euid) };
        if pw.is_not_null() {
            print!(
                "({:s})",
                unsafe { from_c_str(read(pw).pw_name) });
        }
    }

    let egid = unsafe { getegid() };
    if p_egid && (egid != gid as u32) {
        print!(" egid={:u}", egid);
        unsafe {
            let grp = getgrgid(egid);
            if grp.is_not_null() {
                print!("({:s})", from_c_str(read(grp).gr_name));
            }
        }
    }

    unsafe { groups.set_len(ngroups as uint) };

    if ngroups > 0 {
        print!(" groups=");

        let mut first = true;
        for &gr in groups.iter() {
            if !first { print!(",") }
            print!("{:d}", gr);
            let group = unsafe { getgrgid(gr as u32) };
            if group.is_not_null() {
                let name = unsafe {
                    from_c_str(read(group).gr_name)
                };
                print!("({:s})", name);
            }
            first = false
        }

        println!("");
    }
}
