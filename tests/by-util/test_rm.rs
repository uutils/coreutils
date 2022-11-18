use crate::common::util::*;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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

#[test]
fn test_rm_descend_directory() {
    // This test descends into each directory and deletes the files and folders inside of them
    // This test will have the rm process asks 6 question and us answering Y to them will delete all the files and folders

    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let yes = format!("y{}", END_OF_LINE);

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "a/at.txt";
    let file_2 = "a/b/bt.txt";

    at.mkdir_all("a/b/");
    at.touch(file_1);
    at.touch(file_2);

    let mut child = scene.ucmd().arg("-ri").arg("a").run_no_wait();
    child.write_in(yes.as_bytes()).unwrap();
    child.write_in(yes.as_bytes()).unwrap();
    child.write_in(yes.as_bytes()).unwrap();
    child.write_in(yes.as_bytes()).unwrap();
    child.write_in(yes.as_bytes()).unwrap();
    child.write_in(yes.as_bytes()).unwrap();

    child.wait().unwrap();

    assert!(!at.dir_exists("a/b"));
    assert!(!at.dir_exists("a"));
    assert!(!at.file_exists(file_1));
    assert!(!at.file_exists(file_2));
}

#[cfg(feature = "chmod")]
#[test]
fn test_rm_prompts() {
    use std::io::Write;

    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let mut answers = vec![
        "rm: descend into directory 'a'?",
        "rm: remove write-protected regular empty file 'a/empty-no-write'?",
        "rm: remove symbolic link 'a/slink'?",
        "rm: remove symbolic link 'a/slink-dot'?",
        "rm: remove write-protected regular file 'a/f-no-write'?",
        "rm: remove regular empty file 'a/empty'?",
        "rm: remove directory 'a/b'?",
        "rm: remove write-protected directory 'a/b-no-write'?",
        "rm: remove directory 'a'?",
    ];

    answers.sort();

    let yes = format!("y{}", END_OF_LINE);

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("a/");

    let file_1 = "a/empty";
    let file_2 = "a/empty-no-write";
    let file_3 = "a/f-no-write";

    at.touch(file_1);
    at.touch(file_2);
    at.make_file(file_3)
        .write_all(b"not-empty")
        .expect("Couldn't write to a/f-no-write");

    at.symlink_dir("a/empty-f", "a/slink");
    at.symlink_dir(".", "a/slink-dot");

    let dir_1 = "a/b/";
    let dir_2 = "a/b-no-write/";

    at.mkdir(dir_1);
    at.mkdir(dir_2);

    scene
        .ccmd("chmod")
        .arg("u-w")
        .arg(file_3)
        .arg(dir_2)
        .arg(file_2)
        .succeeds();

    let mut child = scene.ucmd().arg("-ri").arg("a").run_no_wait();
    for _ in 0..9 {
        child.write_in(yes.as_bytes()).unwrap();
    }

    let result = child.wait().unwrap();

    let mut trimmed_output = Vec::new();
    for string in result.stderr_str().split("rm: ") {
        if !string.is_empty() {
            let trimmed_string = format!("rm: {}", string).trim().to_string();
            trimmed_output.push(trimmed_string);
        }
    }

    trimmed_output.sort();

    assert_eq!(trimmed_output.len(), answers.len());

    for (i, checking_string) in trimmed_output.iter().enumerate() {
        assert_eq!(checking_string, answers[i]);
    }

    assert!(!at.dir_exists("a"));
}

#[test]
fn test_rm_force_prompts_order() {
    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let yes = format!("y{}", END_OF_LINE);

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let empty_file = "empty";

    at.touch(empty_file);

    // This should cause rm to prompt to remove regular empty file
    let mut child = scene.ucmd().arg("-fi").arg(empty_file).run_no_wait();
    child.write_in(yes.as_bytes()).unwrap();

    let result = child.wait().unwrap();
    let string_output = result.stderr_str();
    assert_eq!(
        string_output.trim(),
        "rm: remove regular empty file 'empty'?"
    );
    assert!(!at.file_exists(empty_file));

    at.touch(empty_file);

    // This should not cause rm to prompt to remove regular empty file
    scene
        .ucmd()
        .arg("-if")
        .arg(empty_file)
        .succeeds()
        .no_stderr();
    assert!(!at.file_exists(empty_file));
}

#[test]
#[ignore = "issue #3722"]
fn test_rm_directory_rights_rm1() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("b/a/p");
    at.mkdir_all("b/c");
    at.mkdir_all("b/d");
    at.set_readonly("b/a");
    ucmd.args(&["-rf", "b"])
        .fails()
        .stderr_contains("Permission denied");
    assert!(at.dir_exists("b/a/p"));
    assert!(!at.dir_exists("b/c"));
    assert!(!at.dir_exists("b/d"));
}

#[cfg(feature = "chmod")]
#[test]
fn test_prompt_write_protected_yes() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_1 = "test_rm_prompt_write_protected_1";

    at.touch(file_1);

    scene.ccmd("chmod").arg("0").arg(file_1).succeeds();

    scene.ucmd().arg(file_1).pipe_in("y").succeeds();
    assert!(!at.file_exists(file_1));
}

#[cfg(feature = "chmod")]
#[test]
fn test_prompt_write_protected_no() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_2 = "test_rm_prompt_write_protected_2";

    at.touch(file_2);

    scene.ccmd("chmod").arg("0").arg(file_2).succeeds();

    scene.ucmd().arg(file_2).pipe_in("n").succeeds();
    assert!(at.file_exists(file_2));
}
