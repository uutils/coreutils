// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore bincode serde utmp runlevel testusr testx

use crate::common::util::TestScenario;

#[cfg(not(target_os = "macos"))]
use bincode::serialize;
use regex::Regex;
#[cfg(not(target_os = "macos"))]
use serde::Serialize;
#[cfg(not(target_os = "macos"))]
use serde_big_array::BigArray;
#[cfg(not(target_os = "macos"))]
use std::fs::File;
#[cfg(not(target_os = "macos"))]
use std::{io::Write, path::PathBuf};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime() {
    TestScenario::new(util_name!())
        .ucmd()
        .succeeds()
        .stdout_contains("load average:")
        .stdout_contains(" up ");

    // Don't check for users as it doesn't show in some CI
}

/// Checks for files without utmpx for which boot time cannot be calculated
#[test]
#[cfg(not(target_os = "openbsd"))]
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
#[cfg(target_os = "linux")]
#[ignore = "disabled until fixed"]
fn test_uptime_with_fifo() {
    // This test can go on forever in the CI in some cases, might need aborting
    // Sometimes writing to the pipe is broken
    let ts = TestScenario::new(util_name!());

    let at = &ts.fixtures;
    at.mkfifo("fifo1");

    let child = ts.ucmd().arg("fifo1").run_no_wait();

    let _ = std::fs::write(at.plus("fifo1"), vec![0; 10]);

    child
        .wait()
        .unwrap()
        .failure()
        .stderr_contains("uptime: couldn't get boot time: Illegal seek")
        .stdout_contains("up ???? days ??:??")
        .stdout_contains("load average");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_non_existent_file() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("file1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: No such file or directory")
        .stdout_contains("up ???? days ??:??");
}

// TODO create a similar test for macos
#[test]
#[cfg(not(any(target_os = "openbsd", target_os = "macos")))]
fn test_uptime_with_file_containing_valid_boot_time_utmpx_record() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    // Regex matches for "up   00::00" ,"up 12 days  00::00", the time can be any valid time and
    // the days can be more than 1 digit or not there. This will match even if the amount of whitespace is
    // wrong between the days and the time.

    let re = Regex::new(r"up [(\d){1,} days]*\d{1,2}:\d\d").unwrap();
    utmp(&at.plus("testx"));
    ts.ucmd()
        .arg("testx")
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
            ut_type: RUN_LVL as i32,
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
            ut_type: USER_PROCESS as i32,
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

        let mut buf = serialize(&utmp).unwrap();
        buf.append(&mut serialize(&utmp1).unwrap());
        buf.append(&mut serialize(&utmp2).unwrap());
        let mut f = File::create(path).unwrap();
        f.write_all(&buf).unwrap();
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_extra_argument() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .arg("a")
        .arg("b")
        .fails()
        .stderr_contains("extra operand 'b'");
}
/// Checks whether uptime displays the correct stderr msg when its called with a directory
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_with_dir() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("dir1");

    ts.ucmd()
        .arg("dir1")
        .fails()
        .stderr_contains("uptime: couldn't get boot time: Is a directory")
        .stdout_contains("up ???? days ??:??");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_uptime_since() {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

    new_ucmd!().arg("--since").succeeds().stdout_matches(&re);
}

#[test]
fn test_failed() {
    new_ucmd!().arg("will-fail").fails();
}
