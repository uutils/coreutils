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
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_shred_remove_a";
    let file_b = "test_shred_remove_b";

    // Create file_a and file_b.
    at.touch(file_a);
    at.touch(file_b);

    // Shred file_a.
    scene.ucmd().arg("-u").arg(file_a).succeeds();

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
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "foo.txt";
    at.write(file, "bar");

    let result = scene.ucmd().arg("-vn25").arg(file).succeeds();

    for pat in PATTERNS {
        result.stderr_contains(pat);
    }
}

#[test]
fn test_random_source_regular_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    // Currently, our block size is 4096. If it changes, this test has to be adapted.
    let mut many_bytes = Vec::with_capacity(4096 * 4);
    for i in 0..4096u32 {
        many_bytes.extend(i.to_le_bytes());
    }
    assert_eq!(many_bytes.len(), 4096 * 4);
    at.write_bytes("source_long", &many_bytes);
    let file = "foo.txt";
    at.write(file, "a");
    scene
        .ucmd()
        .arg("-vn3")
        .arg("--random-source=source_long")
        .arg(file)
        .succeeds()
        .stderr_only("shred: foo.txt: pass 1/3 (random)...\nshred: foo.txt: pass 2/3 (random)...\nshred: foo.txt: pass 3/3 (random)...\n");
    // Should rewrite the file exactly three times
    assert_eq!(at.read_bytes(file), many_bytes[(4096 * 2)..(4096 * 3)]);
}

#[test]
#[ignore = "known issue #7947"]
fn test_random_source_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("source");
    let file = "foo.txt";
    at.write(file, "a");
    scene
        .ucmd()
        .arg("-v")
        .arg("--random-source=source")
        .arg(file)
        .fails()
        .stderr_only("shred: foo.txt: pass 1/3 (random)...\nshred: foo.txt: File write pass failed: Is a directory\n");
}
