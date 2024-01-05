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
fn test_invalid_remove_arg() {
    new_ucmd!().arg("--remove=unknown").fails().code_is(1);
}

#[test]
fn test_missing_remove_arg() {
    new_ucmd!().arg("--remove").fails().code_is(1);
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
fn test_shred_remove_unlink() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_shred_remove_unlink";
    at.touch(file);

    ucmd.arg("--remove=unlink").arg(file).succeeds();

    // File was deleted
    assert!(!at.file_exists(file));
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
    let (at, mut ucmd) = at_and_ucmd!();

    let file = "test_shred_remove_wipesync";
    at.touch(file);

    ucmd.arg("--remove=wipesync").arg(file).succeeds();

    // File was deleted
    assert!(!at.file_exists(file));
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
