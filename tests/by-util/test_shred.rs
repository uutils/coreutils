// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore wipesync

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_invalid_remove_arg() {
    new_ucmd!().arg("--remove=unknown").fails().code_is(1);
}

#[test]
fn test_ambiguous_remove_arg() {
    new_ucmd!().arg("--remove=wip").fails().code_is(1);
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
    assert!(at.read_bytes(file) != file_original_content.as_bytes());
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
    scene.ucmd().arg("-u").arg(file).run();

    // file_a was not deleted because it is readonly.
    assert!(at.file_exists(file));

    // Try shred -u -f.
    scene.ucmd().arg("-u").arg("-f").arg(file).run();

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
