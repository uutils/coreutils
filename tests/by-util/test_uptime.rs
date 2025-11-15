// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore bincode serde utmp runlevel testusr testx
#![allow(clippy::cast_possible_wrap, clippy::unreadable_literal)]

use uutests::at_and_ucmd;
use uutests::util::TestScenario;
use uutests::{new_ucmd, util_name};

use regex::Regex;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_uptime() {
    new_ucmd!()
        .succeeds()
        .stdout_contains("load average:")
        .stdout_contains(" up ");

    // Don't check for users as it doesn't show in some CI
}

/// Checks for files without utmpx records for which boot time cannot be calculated
#[test]
#[cfg(not(any(target_os = "openbsd", target_os = "freebsd")))]
// Disabled for freebsd, since it doesn't use the utmpxname() sys call to change the default utmpx
// file that is accessed using getutxent()
fn test_uptime_for_file_without_utmpx_records() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("file1", "hello");

    ucmd.arg(at.plus_as_string("file1"))
        .fails()
        .stderr_contains("uptime: couldn't get boot time")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");
}

/// Checks whether uptime displays the correct stderr msg when its called with a fifo
#[test]
#[cfg(all(unix, feature = "cp"))]
fn test_uptime_with_fifo() {
    // This test can go on forever in the CI in some cases, might need aborting
    // Sometimes writing to the pipe is broken
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.mkfifo("fifo1");

    at.write("a", "hello");
    // Creating a child process to write to the fifo
    let mut child = ts
        .ccmd("cp")
        .arg(at.plus_as_string("a"))
        .arg(at.plus_as_string("fifo1"))
        .run_no_wait();

    ts.ucmd()
        .arg("fifo1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");

    child.kill();
}

#[test]
#[cfg(not(target_os = "freebsd"))]
fn test_uptime_with_non_existent_file() {
    // Disabled for freebsd, since it doesn't use the utmpxname() sys call to change the default utmpx
    // file that is accessed using getutxent()
    new_ucmd!()
        .arg("file1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: No such file or directory")
        .stdout_contains("up ???? days ??:??");
}

// TODO create a similar test for macos
// This will pass
#[test]
#[cfg(not(any(target_os = "openbsd", target_os = "macos")))]
#[cfg(not(target_env = "musl"))]
#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "Issue #7159 - Test not supported on ARM64 Linux"
)]
#[allow(clippy::too_many_lines, clippy::items_after_statements)]
fn test_uptime_with_file_containing_valid_boot_time_utmpx_record() {
    use bincode::{config, serde::encode_to_vec};
    use serde::Serialize;
    use serde_big_array::BigArray;
    use std::fs::File;
    use std::{io::Write, path::PathBuf};

    // This test will pass for freebsd but we currently don't support changing the utmpx file for
    // freebsd.
    let (at, mut ucmd) = at_and_ucmd!();
    // Regex matches for "up   00::00" ,"up 12 days  00::00", the time can be any valid time and
    // the days can be more than 1 digit or not there. This will match even if the amount of whitespace is
    // wrong between the days and the time.

    let re = Regex::new(r"up [(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    utmp(&at.plus("testx"));

    ucmd.arg("testx")
        .succeeds()
        .stdout_matches(&re)
        .stdout_contains("load average");

    // Helper function to create byte sequences
    fn slice_32(slice: &[u8]) -> [i8; 32] {
        let mut arr: [i8; 32] = [0; 32];

        for (i, val) in slice.iter().enumerate() {
            arr[i] = *val as i8;
        }
        arr
    }

    // Creates a file utmp records of three different types including a valid BOOT_TIME entry
    fn utmp(path: &PathBuf) {
        // Definitions of our utmpx structs
        const BOOT_TIME: i32 = 2;
        const RUN_LVL: i32 = 1;
        const USER_PROCESS: i32 = 7;

        #[derive(Serialize)]
        #[repr(C)]
        pub struct TimeVal {
            pub tv_sec: i32,
            pub tv_usec: i32,
        }

        #[derive(Serialize)]
        #[repr(C)]
        pub struct ExitStatus {
            e_termination: i16,
            e_exit: i16,
        }

        #[derive(Serialize)]
        #[repr(C, align(4))]
        pub struct Utmp {
            pub ut_type: i32,
            pub ut_pid: i32,
            pub ut_line: [i8; 32],
            pub ut_id: [i8; 4],

            pub ut_user: [i8; 32],
            #[serde(with = "BigArray")]
            pub ut_host: [i8; 256],
            pub ut_exit: ExitStatus,
            pub ut_session: i32,
            pub ut_tv: TimeVal,

            pub ut_addr_v6: [i32; 4],
            glibc_reserved: [i8; 20],
        }

        let utmp = Utmp {
            ut_type: BOOT_TIME,
            ut_pid: 0,
            ut_line: slice_32("~".as_bytes()),
            ut_id: [126, 126, 0, 0],
            ut_user: slice_32("reboot".as_bytes()),
            ut_host: [0; 256],
            ut_exit: ExitStatus {
                e_termination: 0,
                e_exit: 0,
            },
            ut_session: 0,
            ut_tv: TimeVal {
                tv_sec: 1716371201,
                tv_usec: 290913,
            },
            ut_addr_v6: [127, 0, 0, 1],
            glibc_reserved: [0; 20],
        };
        let utmp1 = Utmp {
            ut_type: RUN_LVL,
            ut_pid: std::process::id() as i32,
            ut_line: slice_32("~".as_bytes()),
            ut_id: [126, 126, 0, 0],
            ut_user: slice_32("runlevel".as_bytes()),
            ut_host: [0; 256],
            ut_exit: ExitStatus {
                e_termination: 0,
                e_exit: 0,
            },
            ut_session: 0,
            ut_tv: TimeVal {
                tv_sec: 1716371209,
                tv_usec: 162250,
            },
            ut_addr_v6: [0, 0, 0, 0],
            glibc_reserved: [0; 20],
        };
        let utmp2 = Utmp {
            ut_type: USER_PROCESS,
            ut_pid: std::process::id() as i32,
            ut_line: slice_32(":1".as_bytes()),
            ut_id: [126, 126, 0, 0],
            ut_user: slice_32("testusr".as_bytes()),
            ut_host: [0; 256],
            ut_exit: ExitStatus {
                e_termination: 0,
                e_exit: 0,
            },
            ut_session: 0,
            ut_tv: TimeVal {
                tv_sec: 1716371283,
                tv_usec: 858764,
            },
            ut_addr_v6: [0, 0, 0, 0],
            glibc_reserved: [0; 20],
        };

        let config = config::legacy();
        let mut buf = encode_to_vec(utmp, config).unwrap();
        buf.append(&mut encode_to_vec(utmp1, config).unwrap());
        buf.append(&mut encode_to_vec(utmp2, config).unwrap());
        let mut f = File::create(path).unwrap();
        f.write_all(&buf).unwrap();
    }
}

#[test]
fn test_uptime_with_extra_argument() {
    new_ucmd!()
        .arg("a")
        .arg("b")
        .fails()
        .stderr_contains("unexpected value 'b'");
}
/// Checks whether uptime displays the correct stderr msg when its called with a directory
#[test]
fn test_uptime_with_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir1");

    ucmd.arg("dir1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: Is a directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(target_os = "openbsd")]
fn test_uptime_check_users_openbsd() {
    new_ucmd!()
        .args(&["openbsd_utmp"])
        .succeeds()
        .stdout_contains("4 users");
}

#[test]
fn test_uptime_since() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    new_ucmd!().arg("--since").succeeds().stdout_matches(&re);
}
