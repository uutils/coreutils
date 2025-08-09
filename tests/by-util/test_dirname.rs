// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_path_with_trailing_slashes() {
    new_ucmd!()
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega//")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\n");
}

#[test]
fn test_path_without_trailing_slashes() {
    new_ucmd!()
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\n");
}

#[test]
fn test_path_without_trailing_slashes_and_zero() {
    new_ucmd!()
        .arg("-z")
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\u{0}");

    new_ucmd!()
        .arg("--zero")
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\u{0}");
}

#[test]
fn test_repeated_zero() {
    new_ucmd!()
        .arg("--zero")
        .arg("--zero")
        .arg("foo/bar")
        .succeeds()
        .stdout_only("foo\u{0}");
}

#[test]
fn test_root() {
    new_ucmd!().arg("/").succeeds().stdout_is("/\n");
}

#[test]
fn test_pwd() {
    new_ucmd!().arg(".").succeeds().stdout_is(".\n");
}

#[test]
fn test_empty() {
    new_ucmd!().arg("").succeeds().stdout_is(".\n");
}

#[test]
#[cfg(unix)]
fn test_dirname_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // Create a test file with non-UTF-8 bytes in the name
    let non_utf8_bytes = b"test_\xFF\xFE/file.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);

    // Test that dirname handles non-UTF-8 paths without crashing
    let result = new_ucmd!().arg(non_utf8_name).succeeds();

    // Just verify it didn't crash and produced some output
    // The exact output format may vary due to lossy conversion
    let output = result.stdout_str_lossy();
    assert!(!output.is_empty());
    assert!(output.contains("test_"));
}
