// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::util::TestScenario;

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;

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
        .contains(&format!("Usage: {} ls", scenario.bin_path.display(),)));
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
#[cfg(unix)]
fn util_name_single() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let scenario = TestScenario::new("sort");
    symlink_file(&scenario.bin_path, scenario.fixtures.plus("uu-sort")).unwrap();
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

#[test]
#[cfg(unix)]
fn util_invalid_name_help() {
    use std::process::{Command, Stdio};

    let scenario = TestScenario::new("invalid_name");
    symlink_file(&scenario.bin_path, scenario.fixtures.plus("invalid_name")).unwrap();
    let child = Command::new(scenario.fixtures.plus("invalid_name"))
        .arg("--help")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("(multi-call binary)"), "{output_str:?}");
    assert!(
        output_str.contains("Usage: invalid_name [function "),
        "{output_str:?}"
    );
}

#[test]
// The exact set of permitted filenames depends on many factors. Non-UTF-8 strings
// work on very few platforms, but linux works, especially because it also increases
// the likelihood that a filesystem is being used that supports non-UTF-8 filenames.
#[cfg(target_os = "linux")]
fn util_non_utf8_name_help() {
    // Make sure we don't crash even if the util name is invalid UTF-8.
    use std::{
        ffi::OsStr,
        os::unix::ffi::OsStrExt,
        process::{Command, Stdio},
    };

    let scenario = TestScenario::new("invalid_name");
    let non_utf8_path = scenario.fixtures.plus(OsStr::from_bytes(b"\xff"));
    symlink_file(&scenario.bin_path, &non_utf8_path).unwrap();
    let child = Command::new(&non_utf8_path)
        .arg("--help")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("(multi-call binary)"), "{output_str:?}");
    assert!(
        output_str.contains("Usage: <unknown binary name> [function "),
        "{output_str:?}"
    );
}

#[test]
#[cfg(unix)]
fn util_invalid_name_invalid_command() {
    use std::process::{Command, Stdio};

    let scenario = TestScenario::new("invalid_name");
    symlink_file(&scenario.bin_path, scenario.fixtures.plus("invalid_name")).unwrap();
    let child = Command::new(scenario.fixtures.plus("invalid_name"))
        .arg("definitely_invalid")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, b"");
    assert_eq!(
        output.stdout,
        b"definitely_invalid: function/utility not found\n"
    );
}

#[test]
#[cfg(feature = "true")]
fn util_completion() {
    use std::process::{Command, Stdio};

    let scenario = TestScenario::new("completion");
    let child = Command::new(&scenario.bin_path)
        .arg("completion")
        .arg("true")
        .arg("powershell")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(
        output_str.contains("using namespace System.Management.Automation"),
        "{output_str:?}"
    );
}

#[test]
#[cfg(feature = "true")]
fn util_manpage() {
    use std::process::{Command, Stdio};

    let scenario = TestScenario::new("completion");
    let child = Command::new(&scenario.bin_path)
        .arg("manpage")
        .arg("true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("\n.TH true 1 "), "{output_str:?}");
}
