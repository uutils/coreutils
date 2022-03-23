// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) asid auditid auditinfo auid cstr egid emod euid getaudit getlogin gflag nflag pline rflag termid uflag gsflag zflag cflag

// README:
// This was originally based on BSD's `id`
// (noticeable in functionality, usage text, options text, etc.)
// and synced with:
//  http://ftp-archive.freebsd.org/mirror/FreeBSD-Archive/old-releases/i386/1.0-RELEASE/ports/shellutils/src/id.c
//  http://www.opensource.apple.com/source/shell_cmds/shell_cmds-118/id/id.c
//
// * This was partially rewritten in order for stdout/stderr/exit_code
//   to be conform with GNU coreutils (8.32) test suite for `id`.
//
// * This supports multiple users (a feature that was introduced in coreutils 8.31)
//
// * This passes GNU's coreutils Test suite (8.32)
//   for "tests/id/uid.sh" and "tests/id/zero/sh".
//
// * Option '--zero' does not exist for BSD's `id`, therefore '--zero' is only
//   allowed together with other options that are available on GNU's `id`.
//
// * Help text based on BSD's `id` manpage and GNU's `id` manpage.
//
// * This passes GNU's coreutils Test suite (8.32) for "tests/id/context.sh" if compiled with
//   `--features feat_selinux`. It should also pass "tests/id/no-context.sh", but that depends on
//   `uu_ls -Z` being implemented and therefore fails at the moment
//

#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use std::ffi::CStr;
use uucore::display::Quotable;
use uucore::entries::{self, Group, Locate, Passwd};
use uucore::error::UResult;
use uucore::error::{set_exit_code, USimpleError};
use uucore::format_usage;
pub use uucore::libc;
use uucore::libc::{getlogin, uid_t};
use uucore::process::{getegid, geteuid, getgid, getuid};

macro_rules! cstr2cow {
    ($v:expr) => {
        unsafe { CStr::from_ptr($v).to_string_lossy() }
    };
}

static ABOUT: &str = "Print user and group information for each specified USER,
or (when USER omitted) for the current user.";
const USAGE: &str = "{} [OPTION]... [USER]...";

#[cfg(not(feature = "selinux"))]
static CONTEXT_HELP_TEXT: &str = "print only the security context of the process (not enabled)";
#[cfg(feature = "selinux")]
static CONTEXT_HELP_TEXT: &str = "print only the security context of the process";

mod options {
    pub const OPT_AUDIT: &str = "audit"; // GNU's id does not have this
    pub const OPT_CONTEXT: &str = "context";
    pub const OPT_EFFECTIVE_USER: &str = "user";
    pub const OPT_GROUP: &str = "group";
    pub const OPT_GROUPS: &str = "groups";
    pub const OPT_HUMAN_READABLE: &str = "human-readable"; // GNU's id does not have this
    pub const OPT_NAME: &str = "name";
    pub const OPT_PASSWORD: &str = "password"; // GNU's id does not have this
    pub const OPT_REAL_ID: &str = "real";
    pub const OPT_ZERO: &str = "zero"; // BSD's id does not have this
    pub const ARG_USERS: &str = "USER";
}

fn get_description() -> String {
    String::from(
        "The id utility displays the user and group names and numeric IDs, of the \
                      calling process, to the standard output. If the real and effective IDs are \
                      different, both are displayed, otherwise only the real ID is displayed.\n\n\
                      If a user (login name or user ID) is specified, the user and group IDs of \
                      that user are displayed. In this case, the real and effective IDs are \
                      assumed to be the same.",
    )
}

struct Ids {
    uid: u32,  // user id
    gid: u32,  // group id
    euid: u32, // effective uid
    egid: u32, // effective gid
}

struct State {
    nflag: bool,  // --name
    uflag: bool,  // --user
    gflag: bool,  // --group
    gsflag: bool, // --groups
    rflag: bool,  // --real
    zflag: bool,  // --zero
    cflag: bool,  // --context
    selinux_supported: bool,
    ids: Option<Ids>,
    // The behavior for calling GNU's `id` and calling GNU's `id $USER` is similar but different.
    // * The SELinux context is only displayed without a specified user.
    // * The `getgroups` system call is only used without a specified user, this causes
    //   the order of the displayed groups to be different between `id` and `id $USER`.
    //
    // Example:
    // $ strace -e getgroups id -G $USER
    // 1000 10 975 968
    // +++ exited with 0 +++
    // $ strace -e getgroups id -G
    // getgroups(0, NULL)                      = 4
    // getgroups(4, [10, 968, 975, 1000])      = 4
    // 1000 10 968 975
    // +++ exited with 0 +++
    user_specified: bool,
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let after_help = get_description();

    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let users: Vec<String> = matches
        .values_of(options::ARG_USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let mut state = State {
        nflag: matches.is_present(options::OPT_NAME),
        uflag: matches.is_present(options::OPT_EFFECTIVE_USER),
        gflag: matches.is_present(options::OPT_GROUP),
        gsflag: matches.is_present(options::OPT_GROUPS),
        rflag: matches.is_present(options::OPT_REAL_ID),
        zflag: matches.is_present(options::OPT_ZERO),
        cflag: matches.is_present(options::OPT_CONTEXT),

        selinux_supported: {
            #[cfg(feature = "selinux")]
            {
                selinux::kernel_support() != selinux::KernelSupport::Unsupported
            }
            #[cfg(not(feature = "selinux"))]
            {
                false
            }
        },
        user_specified: !users.is_empty(),
        ids: None,
    };

    let default_format = {
        // "default format" is when none of '-ugG' was used
        !(state.uflag || state.gflag || state.gsflag)
    };

    if (state.nflag || state.rflag) && default_format && !state.cflag {
        return Err(USimpleError::new(
            1,
            "cannot print only names or real IDs in default format",
        ));
    }
    if state.zflag && default_format && !state.cflag {
        // NOTE: GNU test suite "id/zero.sh" needs this stderr output:
        return Err(USimpleError::new(
            1,
            "option --zero not permitted in default format",
        ));
    }
    if state.user_specified && state.cflag {
        return Err(USimpleError::new(
            1,
            "cannot print security context when user specified",
        ));
    }

    let delimiter = {
        if state.zflag {
            "\0".to_string()
        } else {
            " ".to_string()
        }
    };
    let line_ending = {
        if state.zflag {
            '\0'
        } else {
            '\n'
        }
    };

    if state.cflag {
        if state.selinux_supported {
            // print SElinux context and exit
            #[cfg(all(target_os = "linux", feature = "selinux"))]
            if let Ok(context) = selinux::SecurityContext::current(false) {
                let bytes = context.as_bytes();
                print!("{}{}", String::from_utf8_lossy(bytes), line_ending);
            } else {
                // print error because `cflag` was explicitly requested
                return Err(USimpleError::new(1, "can't get process context"));
            }
            return Ok(());
        } else {
            return Err(USimpleError::new(
                1,
                "--context (-Z) works only on an SELinux-enabled kernel",
            ));
        }
    }

    for i in 0..=users.len() {
        let possible_pw = if !state.user_specified {
            None
        } else {
            match Passwd::locate(users[i].as_str()) {
                Ok(p) => Some(p),
                Err(_) => {
                    show_error!("{}: no such user", users[i].quote());
                    set_exit_code(1);
                    if i + 1 >= users.len() {
                        break;
                    } else {
                        continue;
                    }
                }
            }
        };

        // GNU's `id` does not support the flags: -p/-P/-A.
        if matches.is_present(options::OPT_PASSWORD) {
            // BSD's `id` ignores all but the first specified user
            pline(possible_pw.as_ref().map(|v| v.uid));
            return Ok(());
        };
        if matches.is_present(options::OPT_HUMAN_READABLE) {
            // BSD's `id` ignores all but the first specified user
            pretty(possible_pw);
            return Ok(());
        }
        if matches.is_present(options::OPT_AUDIT) {
            // BSD's `id` ignores specified users
            auditid();
            return Ok(());
        }

        let (uid, gid) = possible_pw.as_ref().map(|p| (p.uid, p.gid)).unwrap_or((
            if state.rflag { getuid() } else { geteuid() },
            if state.rflag { getgid() } else { getegid() },
        ));
        state.ids = Some(Ids {
            uid,
            gid,
            euid: geteuid(),
            egid: getegid(),
        });

        if state.gflag {
            print!(
                "{}",
                if state.nflag {
                    entries::gid2grp(gid).unwrap_or_else(|_| {
                        show_error!("cannot find name for group ID {}", gid);
                        set_exit_code(1);
                        gid.to_string()
                    })
                } else {
                    gid.to_string()
                }
            );
        }

        if state.uflag {
            print!(
                "{}",
                if state.nflag {
                    entries::uid2usr(uid).unwrap_or_else(|_| {
                        show_error!("cannot find name for user ID {}", uid);
                        set_exit_code(1);
                        uid.to_string()
                    })
                } else {
                    uid.to_string()
                }
            );
        }

        let groups = entries::get_groups_gnu(Some(gid)).unwrap();
        let groups = if state.user_specified {
            possible_pw.as_ref().map(|p| p.belongs_to()).unwrap()
        } else {
            groups.clone()
        };

        if state.gsflag {
            print!(
                "{}{}",
                groups
                    .iter()
                    .map(|&id| {
                        if state.nflag {
                            entries::gid2grp(id).unwrap_or_else(|_| {
                                show_error!("cannot find name for group ID {}", id);
                                set_exit_code(1);
                                id.to_string()
                            })
                        } else {
                            id.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(&delimiter),
                // NOTE: this is necessary to pass GNU's "tests/id/zero.sh":
                if state.zflag && state.user_specified && users.len() > 1 {
                    "\0"
                } else {
                    ""
                }
            );
        }

        if default_format {
            id_print(&mut state, &groups);
        }
        print!("{}", line_ending);

        if i + 1 >= users.len() {
            break;
        }
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_AUDIT)
                .short('A')
                .conflicts_with_all(&[
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_GROUPS,
                    options::OPT_ZERO,
                ])
                .help(
                    "Display the process audit user ID and other process audit properties,\n\
                      which requires privilege (not available on Linux).",
                ),
        )
        .arg(
            Arg::new(options::OPT_EFFECTIVE_USER)
                .short('u')
                .long(options::OPT_EFFECTIVE_USER)
                .conflicts_with(options::OPT_GROUP)
                .help("Display only the effective user ID as a number."),
        )
        .arg(
            Arg::new(options::OPT_GROUP)
                .short('g')
                .long(options::OPT_GROUP)
                .conflicts_with(options::OPT_EFFECTIVE_USER)
                .help("Display only the effective group ID as a number"),
        )
        .arg(
            Arg::new(options::OPT_GROUPS)
                .short('G')
                .long(options::OPT_GROUPS)
                .conflicts_with_all(&[
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_CONTEXT,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_AUDIT,
                ])
                .help(
                    "Display only the different group IDs as white-space separated numbers, \
                      in no particular order.",
                ),
        )
        .arg(
            Arg::new(options::OPT_HUMAN_READABLE)
                .short('p')
                .help("Make the output human-readable. Each display is on a separate line."),
        )
        .arg(
            Arg::new(options::OPT_NAME)
                .short('n')
                .long(options::OPT_NAME)
                .help(
                    "Display the name of the user or group ID for the -G, -g and -u options \
                      instead of the number.\nIf any of the ID numbers cannot be mapped into \
                      names, the number will be displayed as usual.",
                ),
        )
        .arg(
            Arg::new(options::OPT_PASSWORD)
                .short('P')
                .help("Display the id as a password file entry."),
        )
        .arg(
            Arg::new(options::OPT_REAL_ID)
                .short('r')
                .long(options::OPT_REAL_ID)
                .help(
                    "Display the real ID for the -G, -g and -u options instead of \
                      the effective ID.",
                ),
        )
        .arg(
            Arg::new(options::OPT_ZERO)
                .short('z')
                .long(options::OPT_ZERO)
                .help(
                    "delimit entries with NUL characters, not whitespace;\n\
                      not permitted in default format",
                ),
        )
        .arg(
            Arg::new(options::OPT_CONTEXT)
                .short('Z')
                .long(options::OPT_CONTEXT)
                .conflicts_with_all(&[options::OPT_GROUP, options::OPT_EFFECTIVE_USER])
                .help(CONTEXT_HELP_TEXT),
        )
        .arg(
            Arg::new(options::ARG_USERS)
                .multiple_occurrences(true)
                .takes_value(true)
                .value_name(options::ARG_USERS),
        )
}

fn pretty(possible_pw: Option<Passwd>) {
    if let Some(p) = possible_pw {
        print!("uid\t{}\ngroups\t", p.name);
        println!(
            "{}",
            p.belongs_to()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        let login = cstr2cow!(getlogin() as *const _);
        let rid = getuid();
        if let Ok(p) = Passwd::locate(rid) {
            if login == p.name {
                println!("login\t{}", login);
            }
            println!("uid\t{}", p.name);
        } else {
            println!("uid\t{}", rid);
        }

        let eid = getegid();
        if eid == rid {
            if let Ok(p) = Passwd::locate(eid) {
                println!("euid\t{}", p.name);
            } else {
                println!("euid\t{}", eid);
            }
        }

        let rid = getgid();
        if rid != eid {
            if let Ok(g) = Group::locate(rid) {
                println!("euid\t{}", g.name);
            } else {
                println!("euid\t{}", rid);
            }
        }

        println!(
            "groups\t{}",
            entries::get_groups_gnu(None)
                .unwrap()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}

#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or_else(getuid);
    let pw = Passwd::locate(uid).unwrap();

    println!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        pw.name,
        pw.user_passwd,
        pw.uid,
        pw.gid,
        pw.user_access_class,
        pw.passwd_change_time,
        pw.expiration,
        pw.user_info,
        pw.user_dir,
        pw.user_shell
    );
}

#[cfg(target_os = "linux")]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or_else(getuid);
    let pw = Passwd::locate(uid).unwrap();

    println!(
        "{}:{}:{}:{}:{}:{}:{}",
        pw.name, pw.user_passwd, pw.uid, pw.gid, pw.user_info, pw.user_dir, pw.user_shell
    );
}

#[cfg(target_os = "linux")]
fn auditid() {}

#[cfg(not(target_os = "linux"))]
fn auditid() {
    #[allow(deprecated)]
    let mut auditinfo: audit::c_auditinfo_addr_t = unsafe { std::mem::uninitialized() };
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

fn id_print(state: &mut State, groups: &[u32]) {
    let uid = state.ids.as_ref().unwrap().uid;
    let gid = state.ids.as_ref().unwrap().gid;
    let euid = state.ids.as_ref().unwrap().euid;
    let egid = state.ids.as_ref().unwrap().egid;

    print!(
        "uid={}({})",
        uid,
        entries::uid2usr(uid).unwrap_or_else(|_| {
            show_error!("cannot find name for user ID {}", uid);
            set_exit_code(1);
            uid.to_string()
        })
    );
    print!(
        " gid={}({})",
        gid,
        entries::gid2grp(gid).unwrap_or_else(|_| {
            show_error!("cannot find name for group ID {}", gid);
            set_exit_code(1);
            gid.to_string()
        })
    );
    if !state.user_specified && (euid != uid) {
        print!(
            " euid={}({})",
            euid,
            entries::uid2usr(euid).unwrap_or_else(|_| {
                show_error!("cannot find name for user ID {}", euid);
                set_exit_code(1);
                euid.to_string()
            })
        );
    }
    if !state.user_specified && (egid != gid) {
        print!(
            " egid={}({})",
            euid,
            entries::gid2grp(egid).unwrap_or_else(|_| {
                show_error!("cannot find name for group ID {}", egid);
                set_exit_code(1);
                egid.to_string()
            })
        );
    }
    print!(
        " groups={}",
        groups
            .iter()
            .map(|&gr| format!(
                "{}({})",
                gr,
                entries::gid2grp(gr).unwrap_or_else(|_| {
                    show_error!("cannot find name for group ID {}", gr);
                    set_exit_code(1);
                    gr.to_string()
                })
            ))
            .collect::<Vec<_>>()
            .join(",")
    );

    #[cfg(all(target_os = "linux", feature = "selinux"))]
    if state.selinux_supported
        && !state.user_specified
        && std::env::var_os("POSIXLY_CORRECT").is_none()
    {
        // print SElinux context (does not depend on "-Z")
        if let Ok(context) = selinux::SecurityContext::current(false) {
            let bytes = context.as_bytes();
            print!(" context={}", String::from_utf8_lossy(bytes));
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod audit {
    use super::libc::{c_int, c_uint, dev_t, pid_t, uid_t};

    pub type au_id_t = uid_t;
    pub type au_asid_t = pid_t;
    pub type au_event_t = c_uint;
    pub type au_emod_t = c_uint;
    pub type au_class_t = c_int;
    pub type au_flag_t = u64;

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
        pub ai_auid: au_id_t,         // Audit user ID
        pub ai_mask: au_mask_t,       // Audit masks.
        pub ai_termid: au_tid_addr_t, // Terminal ID.
        pub ai_asid: au_asid_t,       // Audit session ID.
        pub ai_flags: au_flag_t,      // Audit session flags
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern "C" {
        pub fn getaudit(auditinfo_addr: *mut c_auditinfo_addr_t) -> c_int;
    }
}
