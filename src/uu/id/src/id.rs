// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) asid auditid auditinfo auid cstr egid rgid emod euid getaudit getlogin gflag nflag pline rflag termid uflag gsflag zflag cflag

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

use clap::{Arg, ArgAction, Command};
use std::ffi::CStr;
use uucore::display::Quotable;
use uucore::entries::{self, Group, Locate, Passwd};
use uucore::error::UResult;
use uucore::error::{USimpleError, set_exit_code};
pub use uucore::libc;
use uucore::libc::{getlogin, uid_t};
use uucore::line_ending::LineEnding;
use uucore::translate;

use uucore::process::{getegid, geteuid, getgid, getuid};
use uucore::{format_usage, show_error};

macro_rules! cstr2cow {
    ($v:expr) => {
        unsafe {
            let ptr = $v;
            // Must be not null to call cstr2cow
            if ptr.is_null() {
                None
            } else {
                Some({ CStr::from_ptr(ptr) }.to_string_lossy())
            }
        }
    };
}

fn get_context_help_text() -> String {
    #[cfg(not(feature = "selinux"))]
    return translate!("id-context-help-disabled");
    #[cfg(feature = "selinux")]
    return translate!("id-context-help-enabled");
}

mod options {
    pub const OPT_IGNORE: &str = "ignore";
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
#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let users: Vec<String> = matches
        .get_many::<String>(options::ARG_USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let mut state = State {
        nflag: matches.get_flag(options::OPT_NAME),
        uflag: matches.get_flag(options::OPT_EFFECTIVE_USER),
        gflag: matches.get_flag(options::OPT_GROUP),
        gsflag: matches.get_flag(options::OPT_GROUPS),
        rflag: matches.get_flag(options::OPT_REAL_ID),
        zflag: matches.get_flag(options::OPT_ZERO),
        cflag: matches.get_flag(options::OPT_CONTEXT),

        selinux_supported: {
            #[cfg(feature = "selinux")]
            {
                uucore::selinux::is_selinux_enabled()
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
            translate!("id-error-names-real-ids-require-flags"),
        ));
    }
    if state.zflag && default_format && !state.cflag {
        // NOTE: GNU test suite "id/zero.sh" needs this stderr output:
        return Err(USimpleError::new(
            1,
            translate!("id-error-zero-not-permitted-default"),
        ));
    }
    if state.user_specified && state.cflag {
        return Err(USimpleError::new(
            1,
            translate!("id-error-cannot-print-context-with-user"),
        ));
    }

    let delimiter = if state.zflag { "\0" } else { " " };
    let line_ending = LineEnding::from_zero_flag(state.zflag);

    if state.cflag {
        return if state.selinux_supported {
            // print SElinux context and exit
            #[cfg(all(any(target_os = "linux", target_os = "android"), feature = "selinux"))]
            if let Ok(context) = selinux::SecurityContext::current(false) {
                let bytes = context.as_bytes();
                print!("{}{line_ending}", String::from_utf8_lossy(bytes));
            } else {
                // print error because `cflag` was explicitly requested
                return Err(USimpleError::new(
                    1,
                    translate!("id-error-cannot-get-context"),
                ));
            }
            Ok(())
        } else {
            Err(USimpleError::new(
                1,
                translate!("id-error-context-selinux-only"),
            ))
        };
    }

    for i in 0..=users.len() {
        let possible_pw = if state.user_specified {
            match Passwd::locate(users[i].as_str()) {
                Ok(p) => Some(p),
                Err(_) => {
                    show_error!(
                        "{}",
                        translate!("id-error-no-such-user",
                                                     "user" => users[i].quote()
                        )
                    );
                    set_exit_code(1);
                    if i + 1 >= users.len() {
                        break;
                    }

                    continue;
                }
            }
        } else {
            None
        };

        // GNU's `id` does not support the flags: -p/-P/-A.
        if matches.get_flag(options::OPT_PASSWORD) {
            // BSD's `id` ignores all but the first specified user
            pline(possible_pw.as_ref().map(|v| v.uid));
            return Ok(());
        }
        if matches.get_flag(options::OPT_HUMAN_READABLE) {
            // BSD's `id` ignores all but the first specified user
            pretty(possible_pw);
            return Ok(());
        }
        if matches.get_flag(options::OPT_AUDIT) {
            // BSD's `id` ignores specified users
            auditid();
            return Ok(());
        }

        let (uid, gid) = possible_pw.as_ref().map_or(
            {
                let use_effective = !state.rflag && (state.uflag || state.gflag || state.gsflag);
                if use_effective {
                    (geteuid(), getegid())
                } else {
                    (getuid(), getgid())
                }
            },
            |p| (p.uid, p.gid),
        );
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
                        show_error!(
                            "{}",
                            translate!("id-error-cannot-find-group-name", "gid" => gid)
                        );
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
                        show_error!(
                            "{}",
                            translate!("id-error-cannot-find-user-name", "uid" => uid)
                        );
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
                                show_error!(
                                    "{}",
                                    translate!("id-error-cannot-find-group-name", "gid" => id)
                                );
                                set_exit_code(1);
                                id.to_string()
                            })
                        } else {
                            id.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(delimiter),
                // NOTE: this is necessary to pass GNU's "tests/id/zero.sh":
                if state.zflag && state.user_specified && users.len() > 1 {
                    "\0"
                } else {
                    ""
                }
            );
        }

        if default_format {
            id_print(&state, &groups);
        }
        print!("{line_ending}");

        if i + 1 >= users.len() {
            break;
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("id-about"))
        .override_usage(format_usage(&translate!("id-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .after_help(translate!("id-after-help"))
        .arg(
            Arg::new(options::OPT_IGNORE)
                .short('a')
                .long(options::OPT_IGNORE)
                .help(translate!("id-help-ignore"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_AUDIT)
                .short('A')
                .conflicts_with_all([
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_GROUPS,
                    options::OPT_ZERO,
                ])
                .help(translate!("id-help-audit"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_EFFECTIVE_USER)
                .short('u')
                .long(options::OPT_EFFECTIVE_USER)
                .conflicts_with(options::OPT_GROUP)
                .help(translate!("id-help-user"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_GROUP)
                .short('g')
                .long(options::OPT_GROUP)
                .conflicts_with(options::OPT_EFFECTIVE_USER)
                .help(translate!("id-help-group"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_GROUPS)
                .short('G')
                .long(options::OPT_GROUPS)
                .conflicts_with_all([
                    options::OPT_GROUP,
                    options::OPT_EFFECTIVE_USER,
                    options::OPT_CONTEXT,
                    options::OPT_HUMAN_READABLE,
                    options::OPT_PASSWORD,
                    options::OPT_AUDIT,
                ])
                .help(translate!("id-help-groups"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_HUMAN_READABLE)
                .short('p')
                .help(translate!("id-help-human-readable"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NAME)
                .short('n')
                .long(options::OPT_NAME)
                .help(translate!("id-help-name"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PASSWORD)
                .short('P')
                .help(translate!("id-help-password"))
                .conflicts_with(options::OPT_HUMAN_READABLE)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_REAL_ID)
                .short('r')
                .long(options::OPT_REAL_ID)
                .help(translate!("id-help-real"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_ZERO)
                .short('z')
                .long(options::OPT_ZERO)
                .help(translate!("id-help-zero"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_CONTEXT)
                .short('Z')
                .long(options::OPT_CONTEXT)
                .conflicts_with_all([options::OPT_GROUP, options::OPT_EFFECTIVE_USER])
                .help(get_context_help_text())
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_USERS)
                .action(ArgAction::Append)
                .value_name(options::ARG_USERS)
                .value_hint(clap::ValueHint::Username),
        )
}

fn pretty(possible_pw: Option<Passwd>) {
    if let Some(p) = possible_pw {
        print!(
            "{}\t{}\n{}\t",
            translate!("id-output-uid"),
            p.name,
            translate!("id-output-groups")
        );
        println!(
            "{}",
            p.belongs_to()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap_or_else(|_| gr.to_string()))
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        let login = cstr2cow!(getlogin().cast_const());
        let uid = getuid();
        if let Ok(p) = Passwd::locate(uid) {
            if let Some(user_name) = login {
                println!("{}\t{user_name}", translate!("id-output-login"));
            }
            println!("{}\t{}", translate!("id-output-uid"), p.name);
        } else {
            println!("{}\t{uid}", translate!("id-output-uid"));
        }

        let euid = geteuid();
        if euid != uid {
            if let Ok(p) = Passwd::locate(euid) {
                println!("{}\t{}", translate!("id-output-euid"), p.name);
            } else {
                println!("{}\t{euid}", translate!("id-output-euid"));
            }
        }

        let rgid = getgid();
        let egid = getegid();
        if egid != rgid {
            if let Ok(g) = Group::locate(rgid) {
                println!("{}\t{}", translate!("id-output-rgid"), g.name);
            } else {
                println!("{}\t{rgid}", translate!("id-output-rgid"));
            }
        }

        println!(
            "{}\t{}",
            translate!("id-output-groups"),
            entries::get_groups_gnu(None)
                .unwrap()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap_or_else(|_| gr.to_string()))
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
        pw.user_passwd.unwrap_or_default(),
        pw.uid,
        pw.gid,
        pw.user_access_class.unwrap_or_default(),
        pw.passwd_change_time,
        pw.expiration,
        pw.user_info.unwrap_or_default(),
        pw.user_dir.unwrap_or_default(),
        pw.user_shell.unwrap_or_default()
    );
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "openbsd",
    target_os = "cygwin"
))]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or_else(getuid);
    let pw = Passwd::locate(uid).unwrap();

    println!(
        "{}:{}:{}:{}:{}:{}:{}",
        pw.name,
        pw.user_passwd.unwrap_or_default(),
        pw.uid,
        pw.gid,
        pw.user_info.unwrap_or_default(),
        pw.user_dir.unwrap_or_default(),
        pw.user_shell.unwrap_or_default()
    );
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "openbsd",
    target_os = "cygwin"
))]
fn auditid() {}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "openbsd",
    target_os = "cygwin"
)))]
fn auditid() {
    use std::mem::MaybeUninit;

    let mut auditinfo: MaybeUninit<audit::c_auditinfo_addr_t> = MaybeUninit::uninit();
    let address = auditinfo.as_mut_ptr();
    if unsafe { audit::getaudit(address) } < 0 {
        println!("{}", translate!("id-error-audit-retrieve"));
        return;
    }

    // SAFETY: getaudit wrote a valid struct to auditinfo
    let auditinfo = unsafe { auditinfo.assume_init() };

    println!("auid={}", auditinfo.ai_auid);
    println!("mask.success=0x{:x}", auditinfo.ai_mask.am_success);
    println!("mask.failure=0x{:x}", auditinfo.ai_mask.am_failure);
    println!("termid.port=0x{:x}", auditinfo.ai_termid.port);
    println!("asid={}", auditinfo.ai_asid);
}

fn id_print(state: &State, groups: &[u32]) {
    let uid = state.ids.as_ref().unwrap().uid;
    let gid = state.ids.as_ref().unwrap().gid;
    let euid = state.ids.as_ref().unwrap().euid;
    let egid = state.ids.as_ref().unwrap().egid;

    print!(
        "uid={uid}({})",
        entries::uid2usr(uid).unwrap_or_else(|_| {
            show_error!(
                "{}",
                translate!("id-error-cannot-find-user-name", "uid" => uid)
            );
            set_exit_code(1);
            uid.to_string()
        })
    );
    print!(
        " gid={gid}({})",
        entries::gid2grp(gid).unwrap_or_else(|_| {
            show_error!(
                "{}",
                translate!("id-error-cannot-find-group-name", "gid" => gid)
            );
            set_exit_code(1);
            gid.to_string()
        })
    );
    if !state.user_specified && (euid != uid) {
        print!(
            " euid={euid}({})",
            entries::uid2usr(euid).unwrap_or_else(|_| {
                show_error!(
                    "{}",
                    translate!("id-error-cannot-find-user-name", "uid" => euid)
                );
                set_exit_code(1);
                euid.to_string()
            })
        );
    }
    if !state.user_specified && (egid != gid) {
        // BUG?  printing egid={euid} ?
        print!(
            " egid={egid}({})",
            entries::gid2grp(egid).unwrap_or_else(|_| {
                show_error!(
                    "{}",
                    translate!("id-error-cannot-find-group-name", "gid" => egid)
                );
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
                "{gr}({})",
                entries::gid2grp(gr).unwrap_or_else(|_| {
                    show_error!(
                        "{}",
                        translate!("id-error-cannot-find-group-name", "gid" => gr)
                    );
                    set_exit_code(1);
                    gr.to_string()
                })
            ))
            .collect::<Vec<_>>()
            .join(",")
    );

    #[cfg(all(any(target_os = "linux", target_os = "android"), feature = "selinux"))]
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

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "openbsd")))]
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
    #[expect(clippy::struct_field_names)]
    pub struct c_auditinfo_addr {
        pub ai_auid: au_id_t,         // Audit user ID
        pub ai_mask: au_mask_t,       // Audit masks.
        pub ai_termid: au_tid_addr_t, // Terminal ID.
        pub ai_asid: au_asid_t,       // Audit session ID.
        pub ai_flags: au_flag_t,      // Audit session flags
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    unsafe extern "C" {
        pub fn getaudit(auditinfo_addr: *mut c_auditinfo_addr_t) -> c_int;
    }
}
