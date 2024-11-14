// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_kmsg_json() {
    new_ucmd!()
        .arg("--kmsg-file")
        .arg("kmsg.input")
        .arg("--json")
        .run()
        .no_stderr()
        .stdout_is_fixture("test_kmsg_json.expected");
}

#[test]
fn test_kmsg_delta() {
    new_ucmd!()
        .arg("-kmsg-file")
        .arg("kmsg.input")
        .arg("--show-delta")
        .run()
        .no_stderr()
        .stdout_is_fixture("test_kmsg_show_delta.expected");
}

#[test]
fn test_kmsg_facility() {
    let facilities = ["kern", "user", "daemon", "syslog"];
    for facility in facilities {
        let facility_arg = format!("--facility={facility}");
        let expected_output_file = format!("test_kmsg_facility_{facility}.expected");
        new_ucmd!()
            .arg("-kmsg-file")
            .arg("kmsg.input")
            .arg(facility_arg)
            .run()
            .no_stderr()
            .stdout_is_fixture(expected_output_file);
    }
}

#[test]
fn test_kmsg_level() {
    let levels = ["err", "warn", "notice", "info"];
    for level in levels {
        let level_arg = format!("--level={level}");
        let expected_output_file = format!("test_kmsg_level_{level}.expected");
        new_ucmd!()
            .arg("-kmsg-file")
            .arg("kmsg.input")
            .arg(level_arg)
            .run()
            .no_stderr()
            .stdout_is_fixture(expected_output_file);
    }
}

#[test]
fn test_kmsg_multiple_facility() {
    new_ucmd!()
        .arg("-kmsg-file")
        .arg("kmsg.input")
        .arg("--facility=\"user,daemon\"")
        .run()
        .no_stderr()
        .stdout_is_fixture("test_kmsg_multiple_facility.expected");
}

#[test]
fn test_kmsg_decode() {
    new_ucmd!()
        .arg("-kmsg-file")
        .arg("kmsg.input")
        .arg("--decode")
        .run()
        .no_stderr()
        .stdout_is_fixture("test_kmsg_decode.expected");
}

#[test]
fn test_kmsg_color() {
    new_ucmd!()
        .arg("-kmsg-file")
        .arg("kmsg.input")
        .arg("--color=always")
        .run()
        .no_stderr()
        .stdout_is_fixture("test_kmsg_color.expected");
}
