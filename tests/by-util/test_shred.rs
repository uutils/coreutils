use crate::common::util::*;

#[test]
fn test_shred_remove() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_shred_remove_a";
    let file_b = "test_shred_remove_b";

    // Create file_a and file_b.
    at.touch(file_a);
    at.touch(file_b);

    // Shred file_a.
    scene.ucmd().arg("-u").arg(file_a).run();

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

    let metadata = at.metadata(file);
    let permissions = metadata.permissions();
    println!("test_shred_force: is file readonly: {}", permissions.readonly());

    // Try shred -u.
    let result = scene.ucmd().arg("-u").arg(file).run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);

    // file_a was not deleted because it is readonly.
    assert!(at.file_exists(file));

    // Try shred -u -f.
    scene.ucmd().arg("-u").arg("-f").arg(file).run();

    // file_a was deleted.
    assert!(!at.file_exists(file));
}
