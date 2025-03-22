// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::io::IsTerminal;
#[cfg(target_family = "unix")]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_more_no_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!().fails().stderr_contains("more: bad usage");
    }
}

#[test]
fn test_valid_arg() {
    if std::io::stdout().is_terminal() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let file = "test_file";
        at.touch(file);

        scene.ucmd().arg(file).arg("-c").succeeds();
        scene.ucmd().arg(file).arg("--print-over").succeeds();

        scene.ucmd().arg(file).arg("-p").succeeds();
        scene.ucmd().arg(file).arg("--clean-print").succeeds();

        scene.ucmd().arg(file).arg("-s").succeeds();
        scene.ucmd().arg(file).arg("--squeeze").succeeds();

        scene.ucmd().arg(file).arg("-u").succeeds();
        scene.ucmd().arg(file).arg("--plain").succeeds();

        scene.ucmd().arg(file).arg("-n").arg("10").succeeds();
        scene.ucmd().arg(file).arg("--lines").arg("0").succeeds();
        scene.ucmd().arg(file).arg("--number").arg("0").succeeds();

        scene.ucmd().arg(file).arg("-F").arg("10").succeeds();
        scene
            .ucmd()
            .arg(file)
            .arg("--from-line")
            .arg("0")
            .succeeds();

        scene.ucmd().arg(file).arg("-P").arg("something").succeeds();
        scene.ucmd().arg(file).arg("--pattern").arg("-1").succeeds();
    }
}

#[test]
fn test_invalid_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!().arg("--invalid").fails();

        new_ucmd!().arg("--lines").arg("-10").fails();
        new_ucmd!().arg("--number").arg("-10").fails();

        new_ucmd!().arg("--from-line").arg("-10").fails();
    }
}

#[test]
fn test_argument_from_file() {
    if std::io::stdout().is_terminal() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let file = "test_file";

        at.write(file, "1\n2");

        // output all lines
        scene
            .ucmd()
            .arg("-F")
            .arg("0")
            .arg(file)
            .succeeds()
            .no_stderr()
            .stdout_contains("1")
            .stdout_contains("2");

        // output only the second line
        scene
            .ucmd()
            .arg("-F")
            .arg("2")
            .arg(file)
            .succeeds()
            .no_stderr()
            .stdout_contains("2")
            .stdout_does_not_contain("1");
    }
}

#[test]
fn test_more_dir_arg() {
    // Run the test only if there's a valid terminal, else do nothing
    // Maybe we could capture the error, i.e. "Device not found" in that case
    // but I am leaving this for later
    if std::io::stdout().is_terminal() {
        new_ucmd!()
            .arg(".")
            .succeeds()
            .stderr_contains("'.' is a directory.");
    }
}

#[test]
#[cfg(target_family = "unix")]
fn test_more_invalid_file_perms() {
    use std::fs::{set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;

    if std::io::stdout().is_terminal() {
        let (at, mut ucmd) = at_and_ucmd!();
        let permissions = Permissions::from_mode(0o244);
        at.make_file("invalid-perms.txt");
        set_permissions(at.plus("invalid-perms.txt"), permissions).unwrap();
        ucmd.arg("invalid-perms.txt")
            .succeeds()
            .stderr_contains("permission denied");
    }
}

#[test]
fn test_more_error_on_single_arg() {
    if std::io::stdout().is_terminal() {
        let ts = TestScenario::new("more");
        ts.fixtures.mkdir_all("folder");
        ts.ucmd()
            .arg("folder")
            .succeeds()
            .stderr_contains("is a directory");
        ts.ucmd()
            .arg("file1")
            .succeeds()
            .stderr_contains("No such file or directory");
    }
}

#[test]
fn test_more_error_on_multiple_files() {
    if std::io::stdout().is_terminal() {
        let ts = TestScenario::new("more");
        ts.fixtures.mkdir_all("folder");
        ts.fixtures.make_file("file1");
        ts.ucmd()
            .arg("folder")
            .arg("file2")
            .arg("file1")
            .succeeds()
            .stderr_contains("folder")
            .stderr_contains("file2")
            .stdout_contains("file1");
        ts.ucmd()
            .arg("file2")
            .arg("file3")
            .succeeds()
            .stderr_contains("file2")
            .stderr_contains("file3");
    }
}

#[test]
fn test_more_pattern_found() {
    if std::io::stdout().is_terminal() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let file = "test_file";

        at.write(file, "line1\nline2");

        // output only the second line "line2"
        scene
            .ucmd()
            .arg("-P")
            .arg("line2")
            .arg(file)
            .succeeds()
            .no_stderr()
            .stdout_does_not_contain("line1")
            .stdout_contains("line2");
    }
}

#[test]
fn test_more_pattern_not_found() {
    if std::io::stdout().is_terminal() {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        let file = "test_file";

        let file_content = "line1\nline2";
        at.write(file, file_content);

        scene
            .ucmd()
            .arg("-P")
            .arg("something")
            .arg(file)
            .succeeds()
            .no_stderr()
            .stdout_contains("Pattern not found")
            .stdout_contains("line1")
            .stdout_contains("line2");
    }
}
