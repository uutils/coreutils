// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore wipesync

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

const PATTERNS: [&str; 22] = [
    "000000", "ffffff", "555555", "aaaaaa", "249249", "492492", "6db6db", "924924", "b6db6d",
    "db6db6", "111111", "222222", "333333", "444444", "666666", "777777", "888888", "999999",
    "bbbbbb", "cccccc", "dddddd", "eeeeee",
];

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_invalid_remove_arg() {
    new_ucmd!().arg("--remove=unknown").fails_with_code(1);
}

#[test]
fn test_ambiguous_remove_arg() {
    new_ucmd!().arg("--remove=wip").fails_with_code(1);
}

#[test]
fn test_shred() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_shred";
    let file_original_content = "test_shred file content";

    at.write(file, file_original_content);

    ucmd.arg(file).succeeds();

    // File exists
    assert!(at.file_exists(file));
    // File is obfuscated
    assert_ne!(at.read_bytes(file), file_original_content.as_bytes());
}

#[test]
fn test_shred_remove() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_shred_remove";
    at.touch(file);

    ucmd.arg("--remove").arg(file).succeeds();

    // File was deleted
    assert!(!at.file_exists(file));
}

#[test]
fn test_shred_remove_unlink() {
    // spell-checker:disable-next-line
    for argument in ["--remove=unlink", "--remove=unlin", "--remove=u"] {
        let (at, mut ucmd) = at_and_ucmd!();
        let file = "test_shred_remove_unlink";
        at.touch(file);
        ucmd.arg(argument).arg(file).succeeds();
        // File was deleted
        assert!(!at.file_exists(file));
    }
}

#[test]
fn test_shred_remove_wipe() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_shred_remove_wipe";
    at.touch(file);

    ucmd.arg("--remove=wipe").arg(file).succeeds();

    // File was deleted
    assert!(!at.file_exists(file));
}

#[test]
fn test_shred_remove_wipesync() {
    // spell-checker:disable-next-line
    for argument in ["--remove=wipesync", "--remove=wipesyn", "--remove=wipes"] {
        let (at, mut ucmd) = at_and_ucmd!();
        let file = "test_shred_remove_wipesync";
        at.touch(file);
        ucmd.arg(argument).arg(file).succeeds();
        // File was deleted
        assert!(!at.file_exists(file));
    }
}

#[test]
fn test_shred_u() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file_a = "test_shred_remove_a";
    let file_b = "test_shred_remove_b";

    // Create file_a and file_b.
    at.touch(file_a);
    at.touch(file_b);

    // Shred file_a.
    ucmd.arg("-u").arg(file_a).succeeds();

    // file_a was deleted, file_b exists.
    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_shred_force() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_shred_force";

    // Create file_a.
    at.touch(file);
    assert!(at.file_exists(file));

    // Make file_a readonly.
    at.set_readonly(file);

    // Try shred -u.
    scene.ucmd().arg("-u").arg(file).fails();

    // file_a was not deleted because it is readonly.
    assert!(at.file_exists(file));

    // Try shred -u -f.
    scene.ucmd().arg("-u").arg("-f").arg(file).succeeds();

    // file_a was deleted.
    assert!(!at.file_exists(file));
}

#[test]
fn test_hex() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_hex";

    at.touch(file);

    ucmd.arg("--size=0x10").arg(file).succeeds();
}

#[test]
fn test_shred_empty() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_shred_remove_a";

    at.touch(file_a);

    // Shred file_a and verify that, as it is empty, it doesn't have "pass 1/3 (random)"
    scene
        .ucmd()
        .arg("-uv")
        .arg(file_a)
        .succeeds()
        .stderr_does_not_contain("1/3 (random)");

    assert!(!at.file_exists(file_a));

    // if the file isn't empty, we should have random
    at.touch(file_a);
    at.write(file_a, "1");
    scene
        .ucmd()
        .arg("-uv")
        .arg(file_a)
        .succeeds()
        .stderr_contains("1/3 (random)");

    assert!(!at.file_exists(file_a));
}

#[test]
#[cfg(all(unix, feature = "chmod"))]
fn test_shred_fail_no_perm() {
    use std::path::Path;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "dir";

    let file = "test_shred_remove_a";

    let binding = Path::new("dir").join(file);
    let path = binding.to_str().unwrap();
    at.mkdir(dir);
    at.touch(path);
    scene.ccmd("chmod").arg("a-w").arg(dir).succeeds();

    scene
        .ucmd()
        .arg("-uv")
        .arg(path)
        .fails()
        .stderr_contains("Couldn't rename to");
}

#[test]
fn test_shred_verbose_no_padding_1() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "foo";
    at.write(file, "non-empty");
    ucmd.arg("-vn1")
        .arg(file)
        .succeeds()
        .stderr_only("shred: foo: pass 1/1 (random)...\n");
}

#[test]
fn test_shred_verbose_no_padding_10() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "foo";
    at.write(file, "non-empty");
    ucmd.arg("-vn10")
        .arg(file)
        .succeeds()
        .stderr_contains("shred: foo: pass 1/10 (random)...\n");
}

#[test]
fn test_all_patterns_present() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "foo.txt";
    at.write(file, "bar");

    let result = ucmd.arg("-vn25").arg(file).succeeds();

    for pat in PATTERNS {
        result.stderr_contains(pat);
    }
}

#[test]
fn test_random_source_regular_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Currently, our block size is 4096. If it changes, this test has to be adapted.
    let mut many_bytes = Vec::with_capacity(4096 * 4);

    for i in 0..4096u32 {
        many_bytes.extend(i.to_le_bytes());
    }

    assert_eq!(many_bytes.len(), 4096 * 4);
    at.write_bytes("source_long", &many_bytes);

    let file = "foo.txt";
    at.write(file, "a");

    ucmd
        .arg("-vn3")
        .arg("--random-source=source_long")
        .arg(file)
        .succeeds()
        .stderr_only("shred: foo.txt: pass 1/3 (random)...\nshred: foo.txt: pass 2/3 (random)...\nshred: foo.txt: pass 3/3 (random)...\n");

    // Should rewrite the file exactly three times
    assert_eq!(at.read_bytes(file), many_bytes[(4096 * 2)..(4096 * 3)]);
}

#[test]
fn test_random_source_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("source");
    let file = "foo.txt";
    at.write(file, "a");

    // The test verifies that shred stops immediately on error instead of continuing
    // Platform differences:
    // - Unix: Error during write ("File write pass failed: Is a directory")
    // - Windows: Error during open ("cannot open random source")
    // Both are correct - key is NOT seeing "pass 2/3" (which proves it stopped)
    ucmd.arg("-v")
        .arg("--random-source=source")
        .arg(file)
        .fails()
        .stderr_does_not_contain("pass 2/3")
        .stderr_does_not_contain("pass 3/3");
}

#[test]
fn test_shred_rename_exhaustion() {
    // GNU: tests/shred/shred-remove.sh
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test");
    at.touch("000");

    scene
        .ucmd()
        .arg("-vu")
        .arg("test")
        .succeeds()
        .stderr_contains("renamed to 0000")
        .stderr_contains("renamed to 001")
        .stderr_contains("renamed to 00")
        .stderr_contains("removed");

    assert!(!at.file_exists("test"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_shred_non_utf8_paths() {
    use std::os::unix::ffi::OsStrExt;
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
    std::fs::write(at.plus(file_name), "test content").unwrap();

    // Test that shred can handle non-UTF-8 filenames
    ts.ucmd().arg(file_name).succeeds();
}

#[test]
fn test_gnu_shred_passes_20() {
    let (at, mut ucmd) = at_and_ucmd!();

    let us_data = vec![0x55; 102400]; // 100K of 'U' bytes
    at.write_bytes("Us", &us_data);

    let file = "f";
    at.write(file, "1"); // Single byte file

    // Test 20 passes with deterministic random source
    // This should produce the exact same sequence as GNU shred
    let result = ucmd
        .arg("-v")
        .arg("-u")
        .arg("-n20")
        .arg("-s4096")
        .arg("--random-source=Us")
        .arg(file)
        .succeeds();

    // Verify the exact pass sequence matches GNU's behavior
    let expected_passes = [
        "pass 1/20 (random)",
        "pass 2/20 (ffffff)",
        "pass 3/20 (924924)",
        "pass 4/20 (888888)",
        "pass 5/20 (db6db6)",
        "pass 6/20 (777777)",
        "pass 7/20 (492492)",
        "pass 8/20 (bbbbbb)",
        "pass 9/20 (555555)",
        "pass 10/20 (aaaaaa)",
        "pass 11/20 (random)",
        "pass 12/20 (6db6db)",
        "pass 13/20 (249249)",
        "pass 14/20 (999999)",
        "pass 15/20 (111111)",
        "pass 16/20 (000000)",
        "pass 17/20 (b6db6d)",
        "pass 18/20 (eeeeee)",
        "pass 19/20 (333333)",
        "pass 20/20 (random)",
    ];

    for pass in expected_passes {
        result.stderr_contains(pass);
    }

    // Also verify removal messages
    result.stderr_contains("removing");
    result.stderr_contains("renamed to 0");
    result.stderr_contains("removed");

    // File should be deleted
    assert!(!at.file_exists(file));
}

#[test]
fn test_gnu_shred_passes_different_counts() {
    let (at, mut ucmd) = at_and_ucmd!();

    let us_data = vec![0x55; 102400];
    at.write_bytes("Us", &us_data);

    let file = "f";
    at.write(file, "1");

    // Test with 19 passes to verify it works for different counts
    let result = ucmd
        .arg("-v")
        .arg("-n19")
        .arg("--random-source=Us")
        .arg(file)
        .succeeds();

    // Should have exactly 19 passes
    for i in 1..=19 {
        result.stderr_contains(format!("pass {i}/19"));
    }

    // First and last should be random
    result.stderr_contains("pass 1/19 (random)");
    result.stderr_contains("pass 19/19 (random)");
}
