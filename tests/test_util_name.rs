#![allow(unused_imports)]
mod common;

use common::util::TestScenario;

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[test]
#[cfg(feature = "ls")]
fn execution_phrase_double() {
    use std::process::Command;

    let scenario = TestScenario::new("ls");
    let output = Command::new(&scenario.bin_path)
        .arg("ls")
        .arg("--some-invalid-arg")
        .output()
        .unwrap();
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(&format!("USAGE:\n    {} ls", scenario.bin_path.display(),)));
}

#[test]
#[cfg(feature = "ls")]
#[cfg(any(unix, windows))]
fn execution_phrase_single() {
    use std::process::Command;

    let scenario = TestScenario::new("ls");
    symlink_file(scenario.bin_path, scenario.fixtures.plus("uu-ls")).unwrap();
    let output = Command::new(scenario.fixtures.plus("uu-ls"))
        .arg("--some-invalid-arg")
        .output()
        .unwrap();
    assert!(String::from_utf8(output.stderr).unwrap().contains(&format!(
        "USAGE:\n    {}",
        scenario.fixtures.plus("uu-ls").display()
    )));
}

#[test]
#[cfg(feature = "sort")]
fn util_name_double() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let scenario = TestScenario::new("sort");
    let mut child = Command::new(&scenario.bin_path)
        .arg("sort")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // input invalid utf8 to cause an error
    child.stdin.take().unwrap().write_all(&[255]).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(String::from_utf8(output.stderr).unwrap().contains("sort: "));
}

#[test]
#[cfg(feature = "sort")]
#[cfg(any(unix, windows))]
fn util_name_single() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let scenario = TestScenario::new("sort");
    symlink_file(scenario.bin_path, scenario.fixtures.plus("uu-sort")).unwrap();
    let mut child = Command::new(scenario.fixtures.plus("uu-sort"))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // input invalid utf8 to cause an error
    child.stdin.take().unwrap().write_all(&[255]).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(String::from_utf8(output.stderr).unwrap().contains(&format!(
        "{}: ",
        scenario.fixtures.plus("uu-sort").display()
    )));
}
