use crate::common::util::*;

#[test]
fn test_rm_one_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_one_file";

    at.touch(file);

    ucmd.arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
}

#[test]
fn test_rm_failed() {
    let (_at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_one_file"; // Doesn't exist

    ucmd.arg(file).fails().stderr_contains(&format!(
        "cannot remove '{}': No such file or directory",
        file
    ));
}

#[test]
fn test_rm_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_multiple_file_a";
    let file_b = "test_rm_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a).arg(file_b).succeeds().no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_interactive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_rm_interactive_file_a";
    let file_b = "test_rm_interactive_file_b";

    at.touch(file_a);
    at.touch(file_b);

    scene
        .ucmd()
        .arg("-i")
        .arg(file_a)
        .arg(file_b)
        .pipe_in("n")
        .succeeds();

    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    scene
        .ucmd()
        .arg("-i")
        .arg(file_a)
        .arg(file_b)
        .pipe_in("Yesh") // spell-checker:disable-line
        .succeeds();

    assert!(!at.file_exists(file_a));
    assert!(at.file_exists(file_b));
}

#[test]
fn test_rm_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    ucmd.arg("-f")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_force_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    ucmd.arg("-f")
        .arg("-f")
        .arg("-f")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_empty_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_empty_directory";

    at.mkdir(dir);

    ucmd.arg("-d").arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rm_empty_directory_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_empty_directory_verbose";

    at.mkdir(dir);

    ucmd.arg("-d")
        .arg("-v")
        .arg(dir)
        .succeeds()
        .stdout_only(format!("removed directory '{}'\n", dir));

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rm_non_empty_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_non_empty_dir";
    let file_a = &format!("{}/test_rm_non_empty_file_a", dir);

    at.mkdir(dir);
    at.touch(file_a);

    ucmd.arg("-d")
        .arg(dir)
        .fails()
        .stderr_contains(&format!("cannot remove '{}': Directory not empty", dir));
    assert!(at.file_exists(file_a));
    assert!(at.dir_exists(dir));
}

#[test]
fn test_rm_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_recursive_directory";
    let file_a = "test_rm_recursive_directory/test_rm_recursive_file_a";
    let file_b = "test_rm_recursive_directory/test_rm_recursive_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("-r").arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_recursive_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_recursive_directory";
    let file_a = "test_rm_recursive_directory/test_rm_recursive_file_a";
    let file_b = "test_rm_recursive_directory/test_rm_recursive_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("-r")
        .arg("-r")
        .arg("-r")
        .arg(dir)
        .succeeds()
        .no_stderr();

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_rm_directory_without_flag() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_directory_without_flag_dir";

    at.mkdir(dir);

    ucmd.arg(dir)
        .fails()
        .stderr_contains(&format!("cannot remove '{}': Is a directory", dir));
}

#[test]
fn test_rm_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_verbose_file_a";
    let file_b = "test_rm_verbose_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("-v")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .stdout_only(format!("removed '{}'\nremoved '{}'\n", file_a, file_b));
}

#[test]
#[cfg(not(windows))]
// on unix symlink_dir is a file
fn test_rm_symlink_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir = "test_rm_symlink_dir_directory";
    let link = "test_rm_symlink_dir_link";

    at.mkdir(dir);
    at.symlink_dir(dir, link);

    ucmd.arg(link).succeeds();
}

#[test]
#[cfg(windows)]
// on windows removing symlink_dir requires "-r" or "-d"
fn test_rm_symlink_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dir = "test_rm_symlink_dir_directory";
    let link = "test_rm_symlink_dir_link";

    at.mkdir(dir);
    at.symlink_dir(dir, link);

    scene
        .ucmd()
        .arg(link)
        .fails()
        .stderr_contains(&format!("cannot remove '{}': Is a directory", link));

    assert!(at.dir_exists(link));

    scene.ucmd().arg("-r").arg(link).succeeds();
}

#[test]
fn test_rm_invalid_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let link = "test_rm_invalid_symlink";

    at.symlink_file(link, link);

    ucmd.arg(link).succeeds();
}

#[test]
fn test_rm_force_no_operand() {
    let mut ucmd = new_ucmd!();

    ucmd.arg("-f").succeeds().no_stderr();
}

#[test]
fn test_rm_no_operand() {
    let ts = TestScenario::new(util_name!());
    ts.ucmd().fails().stderr_is(&format!(
        "{0}: missing operand\nTry '{1} {0} --help' for more information.\n",
        ts.util_name,
        ts.bin_path.to_string_lossy()
    ));
}

#[test]
fn test_rm_verbose_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_verbose_slash_directory";
    let file_a = &format!("{}/test_rm_verbose_slash_file_a", dir);

    at.mkdir(dir);
    at.touch(file_a);

    let file_a_normalized = &format!(
        "{}{}test_rm_verbose_slash_file_a",
        dir,
        std::path::MAIN_SEPARATOR
    );

    ucmd.arg("-r")
        .arg("-f")
        .arg("-v")
        .arg(&format!("{}///", dir))
        .succeeds()
        .stdout_only(format!(
            "removed '{}'\nremoved directory '{}'\n",
            file_a_normalized, dir
        ));

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
}

#[test]
fn test_rm_silently_accepts_presume_input_tty2() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_2 = "test_rm_silently_accepts_presume_input_tty2";

    at.touch(file_2);

    ucmd.arg("---presume-input-tty").arg(file_2).succeeds();

    assert!(!at.file_exists(file_2));
}

#[test]
fn test_rm_interactive_never() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_2 = "test_rm_interactive";

    at.touch(file_2);
    #[cfg(feature = "chmod")]
    scene.ccmd("chmod").arg("0").arg(file_2).succeeds();

    scene
        .ucmd()
        .arg("--interactive=never")
        .arg(file_2)
        .succeeds()
        .stdout_is("");

    assert!(!at.file_exists(file_2));
}
