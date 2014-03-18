#[allow(non_camel_case_types)];

pub use self::utmpx::{DEFAULT_FILE,USER_PROCESS,c_utmp};
#[cfg(target_os = "linux")]
mod utmpx {
    use std::libc;

    pub static DEFAULT_FILE: &'static str = "/var/run/utmp";

    pub static UT_LINESIZE: uint = 32;
    pub static UT_NAMESIZE: uint = 32;
    pub static UT_IDSIZE: uint = 4;
    pub static UT_HOSTSIZE: uint = 256;

    pub static EMPTY: libc::c_short = 0;
    pub static RUN_LVL: libc::c_short = 1;
    pub static BOOT_TIME: libc::c_short = 2;
    pub static NEW_TIME: libc::c_short = 3;
    pub static OLD_TIME: libc::c_short = 4;
    pub static INIT_PROCESS: libc::c_short = 5;
    pub static LOGIN_PROCESS: libc::c_short = 6;
    pub static USER_PROCESS: libc::c_short = 7;
    pub static DEAD_PROCESS: libc::c_short = 8;
    pub static ACCOUNTING: libc::c_short = 9;

    pub struct c_exit_status {
        e_termination: libc::c_short,
        e_exit: libc::c_short,
    }

    pub struct c_utmp {
        ut_type: libc::c_short,
        ut_pid: libc::pid_t,
        ut_line: [libc::c_char, ..UT_LINESIZE],
        ut_id: [libc::c_char, ..UT_IDSIZE],

        ut_user: [libc::c_char, ..UT_NAMESIZE],
        ut_host: [libc::c_char, ..UT_HOSTSIZE],
        ut_exit: c_exit_status,
        ut_session: libc::c_long,
        ut_tv: libc::timeval,

        ut_addr_v6: [libc::int32_t, ..4],
        __unused: [libc::c_char, ..20],
    }
}

#[cfg(target_os = "macos")]
mod utmpx {
    use std::libc;

    pub static DEFAULT_FILE: &'static str = "/var/run/utmpx";

    pub static UT_LINESIZE: uint = 32;
    pub static UT_NAMESIZE: uint = 256;
    pub static UT_IDSIZE: uint = 4;
    pub static UT_HOSTSIZE: uint = 256;

    pub static EMPTY: libc::c_short = 0;
    pub static RUN_LVL: libc::c_short = 1;
    pub static BOOT_TIME: libc::c_short = 2;
    pub static OLD_TIME: libc::c_short = 3;
    pub static NEW_TIME: libc::c_short = 4;
    pub static INIT_PROCESS: libc::c_short = 5;
    pub static LOGIN_PROCESS: libc::c_short = 6;
    pub static USER_PROCESS: libc::c_short = 7;
    pub static DEAD_PROCESS: libc::c_short = 8;
    pub static ACCOUNTING: libc::c_short = 9;

    pub struct c_exit_status {
        e_termination: libc::c_short,
        e_exit: libc::c_short,
    }

    pub struct c_utmp {
        ut_user: [libc::c_char, ..UT_NAMESIZE],
        ut_id: [libc::c_char, ..UT_IDSIZE],
        ut_line: [libc::c_char, ..UT_LINESIZE],
        ut_pid: libc::pid_t,
        ut_type: libc::c_short,
        ut_tv: libc::timeval,
        ut_host: [libc::c_char, ..UT_HOSTSIZE],
        __unused: [libc::c_char, ..16]
    }
}

