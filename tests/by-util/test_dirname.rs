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

#[test]
fn test_emoji_handling() {
    new_ucmd!()
        .arg("/🌍/path/to/🦀.txt")
        .succeeds()
        .stdout_is("/🌍/path/to\n");

    new_ucmd!()
        .arg("/🎉/path/to/🚀/")
        .succeeds()
        .stdout_is("/🎉/path/to\n");

    new_ucmd!()
        .args(&["-z", "/🌟/emoji/path/🦋.file"])
        .succeeds()
        .stdout_is("/🌟/emoji/path\u{0}");
}

#[test]
fn test_trailing_dot() {
    // Basic case: path ending with /. should return parent without stripping last component
    // This matches GNU coreutils behavior and fixes issue #8910
    new_ucmd!()
        .arg("/home/dos/.")
        .succeeds()
        .stdout_is("/home/dos\n");

    // Root with dot
    new_ucmd!().arg("/.").succeeds().stdout_is("/\n");

    // Relative path with /.
    new_ucmd!().arg("hello/.").succeeds().stdout_is("hello\n");

    // Deeper path with /.
    new_ucmd!()
        .arg("/foo/bar/baz/.")
        .succeeds()
        .stdout_is("/foo/bar/baz\n");
}

#[test]
fn test_trailing_dot_with_zero_flag() {
    // Test that -z flag works correctly with /. paths
    new_ucmd!()
        .arg("-z")
        .arg("/home/dos/.")
        .succeeds()
        .stdout_is("/home/dos\u{0}");

    new_ucmd!()
        .arg("--zero")
        .arg("/.")
        .succeeds()
        .stdout_is("/\u{0}");
}

#[test]
fn test_trailing_dot_multiple_paths() {
    // Test multiple paths, some with /. suffix
    new_ucmd!()
        .args(&["/home/dos/.", "/var/log", "/tmp/."])
        .succeeds()
        .stdout_is("/home/dos\n/var\n/tmp\n");
}

#[test]
fn test_trailing_dot_edge_cases() {
    // Double slash before dot (should still work)
    new_ucmd!()
        .arg("/home/dos//.")
        .succeeds()
        .stdout_is("/home/dos/\n");

    // Path with . in middle (should use normal logic)
    new_ucmd!()
        .arg("/path/./to/file")
        .succeeds()
        .stdout_is("/path/./to\n");
}

#[test]
fn test_trailing_dot_emoji() {
    // Emoji paths with /. suffix
    new_ucmd!()
        .arg("/🌍/path/.")
        .succeeds()
        .stdout_is("/🌍/path\n");

    new_ucmd!().arg("/🎉/🚀/.").succeeds().stdout_is("/🎉/🚀\n");
}

#[test]
#[cfg(unix)]
fn test_trailing_dot_non_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // Create a path with non-UTF-8 bytes ending in /.
    let non_utf8_bytes = b"/test_\xFF\xFE/.";
    let non_utf8_path = OsStr::from_bytes(non_utf8_bytes);

    // Test that dirname handles non-UTF-8 paths with /. suffix
    let result = new_ucmd!().arg(non_utf8_path).succeeds();

    // The output should be the path without the /. suffix
    let output = result.stdout_str_lossy();
    assert!(!output.is_empty());
    assert!(output.contains("test_"));
    // Should not contain the . at the end
    assert!(!output.trim().ends_with('.'));
}

#[test]
fn test_existing_behavior_preserved() {
    // Ensure we didn't break existing test cases
    // These tests verify backward compatibility

    // Normal paths without /. should work as before
    new_ucmd!().arg("/home/dos").succeeds().stdout_is("/home\n");

    new_ucmd!()
        .arg("/home/dos/")
        .succeeds()
        .stdout_is("/home\n");

    // Parent directory references
    new_ucmd!()
        .arg("/home/dos/..")
        .succeeds()
        .stdout_is("/home/dos\n");
}
