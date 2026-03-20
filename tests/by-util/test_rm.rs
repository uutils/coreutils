// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore rootlink ENOTDIR
#![allow(clippy::stable_sort_primitive)]

use std::process::Stdio;

use uutests::{at_and_ucmd, new_ucmd, util::TestScenario, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_one_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_one_file";

    at.touch(file);

    ucmd.arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
}

#[test]
fn test_failed() {
    let file = "test_rm_one_file"; // Doesn't exist

    new_ucmd!()
        .arg(file)
        .fails()
        .stderr_contains(format!("cannot remove '{file}': No such file or directory"));
}

#[test]
fn test_multiple_files() {
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
fn test_interactive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "test_rm_interactive_file_a";
    let file_b = "test_rm_interactive_file_b";

    at.touch(file_a);
    at.touch(file_b);
    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

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
fn test_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    at.touch(file_a);
    at.touch(file_b);
    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

    ucmd.arg("-f")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .no_stderr();

    assert!(!at.file_exists(file_a));
    assert!(!at.file_exists(file_b));
}

#[test]
fn test_force_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_force_a";
    let file_b = "test_rm_force_b";

    at.touch(file_a);
    at.touch(file_b);
    assert!(at.file_exists(file_a));
    assert!(at.file_exists(file_b));

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
fn test_empty_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_empty_directory";

    at.mkdir(dir);

    ucmd.arg("-d").arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_empty_directory_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_empty_directory_verbose";

    at.mkdir(dir);

    ucmd.arg("-d")
        .arg("-v")
        .arg(dir)
        .succeeds()
        .stdout_only(format!("removed directory '{dir}'\n"));

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_non_empty_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_non_empty_dir";
    let file_a = &format!("{dir}/test_rm_non_empty_file_a");

    at.mkdir(dir);
    at.touch(file_a);

    #[cfg(windows)]
    let expected = "rm: cannot remove 'test_rm_non_empty_dir': The directory is not empty.\n";
    #[cfg(not(windows))]
    let expected = "rm: cannot remove 'test_rm_non_empty_dir': Directory not empty\n";
    ucmd.arg("-d").arg(dir).fails().stderr_only(expected);
    assert!(at.file_exists(file_a));
    assert!(at.dir_exists(dir));
}

#[test]
fn test_recursive() {
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
fn test_recursive_multiple() {
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

#[cfg(target_os = "linux")]
#[test]
fn test_recursive_long_filepath() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_recursive_directory";
    let mkdir = "test_rm_recursive_directory/".repeat(35);
    let file_a = mkdir.clone() + "test_rm_recursive_file_a";
    assert!(file_a.len() > 1000);

    at.mkdir_all(&mkdir);
    at.touch(&file_a);

    ucmd.arg("-r").arg(dir).succeeds().no_stderr();

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
}

#[test]
fn test_directory_without_flag() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_directory_without_flag_dir";

    at.mkdir(dir);

    ucmd.arg(dir)
        .fails()
        .stderr_contains(format!("cannot remove '{dir}': Is a directory"));
}

#[test]
#[cfg(windows)]
// https://github.com/uutils/coreutils/issues/3200
fn test_directory_with_trailing_backslash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "dir";

    at.mkdir(dir);

    ucmd.arg(".\\dir\\").arg("-rf").succeeds();
    assert!(!at.dir_exists(dir));
}

#[test]
fn test_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_rm_verbose_file_a";
    let file_b = "test_rm_verbose_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg("-v")
        .arg(file_a)
        .arg(file_b)
        .succeeds()
        .stdout_only(format!("removed '{file_a}'\nremoved '{file_b}'\n"));
}

#[test]
#[cfg(not(windows))]
// on unix symlink_dir is a file
fn test_symlink_dir() {
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
fn test_symlink_dir() {
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
        .stderr_contains(format!("cannot remove '{link}': Is a directory"));

    assert!(at.dir_exists(link));

    scene.ucmd().arg("-r").arg(link).succeeds();
}

#[test]
fn test_invalid_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    let link = "test_rm_invalid_symlink";

    at.symlink_file(link, link);

    ucmd.arg(link).succeeds();
}

#[test]
fn test_force_no_operand() {
    new_ucmd!().arg("-f").succeeds().no_stderr();
}

#[test]
fn test_no_operand() {
    new_ucmd!().fails().usage_error("missing operand");
}

#[test]
fn test_verbose_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_verbose_slash_directory";
    let file_a = &format!("{dir}/test_rm_verbose_slash_file_a");

    at.mkdir(dir);
    at.touch(file_a);

    let file_a_normalized = &format!(
        "{dir}{}test_rm_verbose_slash_file_a",
        std::path::MAIN_SEPARATOR
    );

    ucmd.arg("-r")
        .arg("-f")
        .arg("-v")
        .arg(format!("{dir}///"))
        .succeeds()
        .stdout_only(format!(
            "removed '{file_a_normalized}'\nremoved directory '{dir}'\n"
        ));

    assert!(!at.dir_exists(dir));
    assert!(!at.file_exists(file_a));
}

#[test]
fn test_silently_accepts_presume_input_tty2() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_2 = "test_rm_silently_accepts_presume_input_tty2";

    at.touch(file_2);

    ucmd.arg("---presume-input-tty").arg(file_2).succeeds();

    assert!(!at.file_exists(file_2));
}

#[test]
fn test_interactive_never() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "a";

    for arg in ["never", "no", "none"] {
        at.touch(file);
        #[cfg(feature = "chmod")]
        scene.ccmd("chmod").arg("0").arg(file).succeeds();

        scene
            .ucmd()
            .arg(format!("--interactive={arg}"))
            .arg(file)
            .succeeds()
            .no_output();

        assert!(!at.file_exists(file));
    }
}

#[test]
fn test_interactive_always() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "a";
    let file_b = "b";

    for arg in [
        "-i",
        "--interactive",
        "--interactive=always",
        "--interactive=yes",
    ] {
        at.touch(file_a);
        at.touch(file_b);

        scene
            .ucmd()
            .arg(arg)
            .arg(file_a)
            .arg(file_b)
            .pipe_in("y\ny")
            .succeeds()
            .no_stdout();

        assert!(!at.file_exists(file_a));
        assert!(!at.file_exists(file_b));
    }
}

#[test]
fn test_interactive_once_prompt() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "test_rm_interactive_once_recursive_prompt_file1";
    let file2 = "test_rm_interactive_once_recursive_prompt_file2";
    let file3 = "test_rm_interactive_once_recursive_prompt_file3";
    let file4 = "test_rm_interactive_once_recursive_prompt_file4";

    at.touch(file1);
    at.touch(file2);
    at.touch(file3);
    at.touch(file4);

    ucmd.arg("--interactive=once")
        .arg(file1)
        .arg(file2)
        .arg(file3)
        .arg(file4)
        .pipe_in("y")
        .succeeds()
        .stderr_contains("remove 4 arguments?");

    assert!(!at.file_exists(file1));
    assert!(!at.file_exists(file2));
    assert!(!at.file_exists(file3));
    assert!(!at.file_exists(file4));
}

#[test]
fn test_interactive_once_recursive_prompt() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "test_rm_interactive_once_recursive_prompt_file1";

    at.touch(file1);

    ucmd.arg("--interactive=once")
        .arg("-r")
        .arg(file1)
        .pipe_in("y")
        .succeeds()
        .stderr_contains("remove 1 argument recursively?");

    assert!(!at.file_exists(file1));
}

#[test]
fn test_descend_directory() {
    // This test descends into each directory and deletes the files and folders inside of them
    // This test will have the rm process asks 6 question and us answering Y to them will delete all the files and folders

    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let yes = format!("y{END_OF_LINE}");

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_1 = "a/at.txt";
    let file_2 = "a/b/bt.txt";

    at.mkdir_all("a/b/");
    at.touch(file_1);
    at.touch(file_2);

    let mut child = scene
        .ucmd()
        .set_stdin(Stdio::piped())
        .arg("-ri")
        .arg("a")
        .run_no_wait();
    child.try_write_in(yes.as_bytes()).unwrap();
    child.try_write_in(yes.as_bytes()).unwrap();
    child.try_write_in(yes.as_bytes()).unwrap();
    child.try_write_in(yes.as_bytes()).unwrap();
    child.try_write_in(yes.as_bytes()).unwrap();
    child.try_write_in(yes.as_bytes()).unwrap();

    child.wait().unwrap();

    assert!(!at.dir_exists("a/b"));
    assert!(!at.dir_exists("a"));
    assert!(!at.file_exists(file_1));
    assert!(!at.file_exists(file_2));
}

#[cfg(feature = "chmod")]
#[test]
fn test_prompts() {
    use std::io::Write;

    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let mut answers = [
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

    let yes = format!("y{END_OF_LINE}");

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

    let mut child = scene
        .ucmd()
        .set_stdin(Stdio::piped())
        .arg("-ri")
        .arg("a")
        .run_no_wait();
    for _ in 0..9 {
        child.try_write_in(yes.as_bytes()).unwrap();
    }

    let result = child.wait().unwrap();

    let mut trimmed_output = Vec::new();
    for string in result.stderr_str().split("rm: ") {
        if !string.is_empty() {
            let trimmed_string = format!("rm: {string}").trim().to_string();
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

#[cfg(feature = "chmod")]
#[test]
fn test_prompts_no_tty() {
    // This test ensures InteractiveMode.PromptProtected proceeds silently with non-interactive stdin

    use std::io::Write;

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

    scene.ucmd().arg("-r").arg("a").succeeds().no_output();

    assert!(!at.dir_exists("a"));
}

#[test]
fn test_force_prompts_order() {
    // Needed for talking with stdin on platforms where CRLF or LF matters
    const END_OF_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    let yes = format!("y{END_OF_LINE}");

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let empty_file = "empty";

    at.touch(empty_file);

    // This should cause rm to prompt to remove regular empty file
    let mut child = scene
        .ucmd()
        .set_stdin(Stdio::piped())
        .arg("-fi")
        .arg(empty_file)
        .run_no_wait();
    child.try_write_in(yes.as_bytes()).unwrap();

    let result = child.wait().unwrap();
    result.stderr_only("rm: remove regular empty file 'empty'? ");

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
fn test_directory_rights_rm1() {
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

    scene
        .ucmd()
        .arg("---presume-input-tty")
        .arg(file_1)
        .pipe_in("y")
        .succeeds()
        .stderr_contains("rm: remove write-protected regular empty file");
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

    scene
        .ucmd()
        .arg("---presume-input-tty")
        .arg(file_2)
        .pipe_in("n")
        .succeeds()
        .stderr_contains("rm: remove write-protected regular empty file");
    assert!(at.file_exists(file_2));
}

#[cfg(feature = "chmod")]
#[test]
fn test_remove_inaccessible_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir_1 = "test_rm_protected";

    at.mkdir(dir_1);

    scene.ccmd("chmod").arg("0").arg(dir_1).succeeds();

    scene.ucmd().arg("-rf").arg(dir_1).succeeds();
    assert!(!at.dir_exists(dir_1));
}

#[test]
#[cfg(not(windows))]
fn test_current_or_parent_dir_rm4() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("d");

    let file_1 = "file1";
    let file_2 = "d/file2";

    at.touch(file_1);
    at.touch(file_2);

    let answers = [
        "rm: refusing to remove '.' or '..' directory: skipping 'd/.'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd/./'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd/./'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd/..'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd/../'",
        "rm: refusing to remove '.' or '..' directory: skipping '.'",
        "rm: refusing to remove '.' or '..' directory: skipping './'",
        "rm: refusing to remove '.' or '..' directory: skipping '../'",
        "rm: refusing to remove '.' or '..' directory: skipping '..'",
    ];
    let std_err_str = ts
        .ucmd()
        .arg("-rf")
        .arg("d/.")
        .arg("d/./")
        .arg("d/.////")
        .arg("d/..")
        .arg("d/../")
        .arg(".")
        .arg("./")
        .arg("../")
        .arg("..")
        .fails()
        .stderr_move_str();

    for (idx, line) in std_err_str.lines().enumerate() {
        assert_eq!(line, answers[idx]);
    }
    // checks that no file was silently removed
    assert!(at.dir_exists("d"));
    assert!(at.file_exists(file_1));
    assert!(at.file_exists(file_2));
}

#[test]
#[cfg(windows)]
fn test_current_or_parent_dir_rm4_windows() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("d");

    let file_1 = "file1";
    let file_2 = "d/file2";

    at.touch(file_1);
    at.touch(file_2);

    let answers = [
        "rm: refusing to remove '.' or '..' directory: skipping 'd\\.'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd\\.\\'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd\\.\\'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd\\..'",
        "rm: refusing to remove '.' or '..' directory: skipping 'd\\..\\'",
        "rm: refusing to remove '.' or '..' directory: skipping '.'",
        "rm: refusing to remove '.' or '..' directory: skipping '.\\'",
        "rm: refusing to remove '.' or '..' directory: skipping '..'",
        "rm: refusing to remove '.' or '..' directory: skipping '..\\'",
    ];
    let std_err_str = ts
        .ucmd()
        .arg("-rf")
        .arg("d\\.")
        .arg("d\\.\\")
        .arg("d\\.\\\\\\\\")
        .arg("d\\..")
        .arg("d\\..\\")
        .arg(".")
        .arg(".\\")
        .arg("..")
        .arg("..\\")
        .fails()
        .stderr_move_str();

    for (idx, line) in std_err_str.lines().enumerate() {
        assert_eq!(line, answers[idx]);
    }

    // checks that no file was silently removed
    assert!(at.dir_exists("d"));
    assert!(at.file_exists(file_1));
    assert!(at.file_exists(file_2));
}

#[test]
#[cfg(not(windows))]
fn test_fifo_removal() {
    use std::time::Duration;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkfifo("some_fifo");

    scene
        .ucmd()
        .arg("some_fifo")
        .timeout(Duration::from_secs(2))
        .succeeds();
}

#[test]
fn test_uchild_when_run_no_wait_with_a_blocking_command() {
    let ts = TestScenario::new("rm");
    let at = &ts.fixtures;

    at.mkdir("a");
    at.touch("a/empty");

    #[cfg(target_vendor = "apple")]
    let delay: u64 = 2000;
    #[cfg(not(target_vendor = "apple"))]
    let delay: u64 = 1000;

    let yes = if cfg!(windows) { "y\r\n" } else { "y\n" };

    let mut child = ts
        .ucmd()
        .set_stdin(Stdio::piped())
        .stderr_to_stdout()
        .args(&["-riv", "a"])
        .run_no_wait();
    child
        .make_assertion_with_delay(delay)
        .is_alive()
        .with_current_output()
        .stdout_is("rm: descend into directory 'a'? ");

    #[cfg(windows)]
    let expected = "rm: descend into directory 'a'? \
                    rm: remove regular empty file 'a\\empty'? ";
    #[cfg(unix)]
    let expected = "rm: descend into directory 'a'? \
                    rm: remove regular empty file 'a/empty'? ";
    child.write_in(yes);
    child
        .make_assertion_with_delay(delay)
        .is_alive()
        .with_all_output()
        .stdout_is(expected);

    #[cfg(windows)]
    let expected = "removed 'a\\empty'\nrm: remove directory 'a'? ";
    #[cfg(unix)]
    let expected = "removed 'a/empty'\nrm: remove directory 'a'? ";

    child
        .write_in(yes)
        .make_assertion_with_delay(delay)
        .is_alive()
        .with_exact_output(44, 0)
        .stdout_only(expected);

    let expected = "removed directory 'a'\n";

    child.write_in(yes);
    child.wait().unwrap().stdout_only(expected).success();
}

#[test]
fn test_recursive_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    #[cfg(windows)]
    let expected =
        "rm: descend into directory 'a'? rm: remove directory 'a\\b'? rm: remove directory 'a'? ";
    #[cfg(not(windows))]
    let expected =
        "rm: descend into directory 'a'? rm: remove directory 'a/b'? rm: remove directory 'a'? ";
    ucmd.args(&["-i", "-r", "a"])
        .pipe_in("y\ny\ny\n")
        .succeeds()
        .stderr_only(expected);
    assert!(!at.dir_exists("a/b"));
    assert!(!at.dir_exists("a"));
}

// Avoid an infinite recursion due to a symbolic link to the current directory.
#[test]
fn test_recursive_symlink_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("d");
    at.relative_symlink_file(".", "d/link");
    #[cfg(windows)]
    let expected = "rm: descend into directory 'd'? rm: remove symbolic link 'd\\link'? rm: remove directory 'd'? ";
    #[cfg(not(windows))]
    let expected = "rm: descend into directory 'd'? rm: remove symbolic link 'd/link'? rm: remove directory 'd'? ";
    ucmd.args(&["-i", "-r", "d"])
        .pipe_in("y\ny\ny\n")
        .succeeds()
        .stderr_only(expected);
    assert!(!at.symlink_exists("d/link"));
    assert!(!at.dir_exists("d"));
}

#[cfg(not(windows))]
#[test]
fn test_only_first_error_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.touch("a/b/file");
    // Make the inner directory not writable.
    at.set_mode("a/b", 0o0555);

    // To match the behavior of GNU `rm`, print an error message for
    // the file in the non-writable directory, but don't print the
    // error messages that would have appeared when trying to remove
    // the directories containing the file.
    ucmd.args(&["-r", "-f", "a"])
        .fails()
        .stderr_only("rm: cannot remove 'a/b/file': Permission denied\n");
    assert!(at.file_exists("a/b/file"));
    assert!(at.dir_exists("a/b"));
    assert!(at.dir_exists("a"));
}

#[cfg(not(windows))]
#[test]
fn test_unreadable_and_nonempty_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("a/b");
    at.set_mode("a", 0o0333);

    ucmd.args(&["-r", "-f", "a"])
        .fails()
        .stderr_only("rm: cannot remove 'a': Permission denied\n");
    assert!(at.dir_exists("a/b"));
    assert!(at.dir_exists("a"));
}

#[cfg(not(windows))]
#[test]
fn test_recursive_remove_unreadable_subdir() {
    // Regression test for https://github.com/uutils/coreutils/issues/10966
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("foo/bar");
    at.touch("foo/bar/baz");
    at.set_mode("foo/bar", 0o0000);

    let result = ucmd.args(&["-r", "-f", "foo"]).fails();
    result.stderr_contains("Permission denied");
    result.stderr_contains("foo/bar");

    at.set_mode("foo/bar", 0o0755);
}

#[cfg(not(windows))]
#[test]
fn test_inaccessible_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.set_mode("dir", 0o0333);
    ucmd.args(&["-d", "dir"]).succeeds().no_output();
    assert!(!at.dir_exists("dir"));
}

#[cfg(not(windows))]
#[test]
fn test_inaccessible_dir_nonempty() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.touch("dir/f");
    at.set_mode("dir", 0o0333);
    ucmd.args(&["-d", "dir"])
        .fails()
        .stderr_only("rm: cannot remove 'dir': Directory not empty\n");
    assert!(at.file_exists("dir/f"));
    assert!(at.dir_exists("dir"));
}

#[cfg(not(windows))]
#[test]
fn test_inaccessible_dir_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.set_mode("dir", 0);
    ucmd.args(&["-i", "-d", "dir"])
        .pipe_in("y\n")
        .succeeds()
        .stderr_only("rm: attempt removal of inaccessible directory 'dir'? ");
    assert!(!at.dir_exists("dir"));
}

#[cfg(not(windows))]
#[test]
fn test_inaccessible_dir_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/unreadable");
    at.set_mode("a/unreadable", 0o0333);
    ucmd.args(&["-r", "-f", "a"]).succeeds().no_output();
    assert!(!at.dir_exists("a/unreadable"));
    assert!(!at.dir_exists("a"));
}

#[test]
#[cfg(any(target_os = "linux", target_os = "wasi"))]
fn test_non_utf8_paths() {
    use std::ffi::OsStr;
    #[cfg(target_os = "linux")]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a test file with non-UTF-8 bytes in the name
    let non_utf8_bytes = b"test_\xFF\xFE.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);

    // Create the actual file
    at.touch(non_utf8_name);
    assert!(at.file_exists(non_utf8_name));

    // Test that rm handles non-UTF-8 file names without crashing
    scene.ucmd().arg(non_utf8_name).succeeds();

    // The file should be removed
    assert!(!at.file_exists(non_utf8_name));

    // Test with directory
    let non_utf8_dir_bytes = b"test_dir_\xFF\xFE";
    let non_utf8_dir_name = OsStr::from_bytes(non_utf8_dir_bytes);

    at.mkdir(non_utf8_dir_name);
    assert!(at.dir_exists(non_utf8_dir_name));

    scene.ucmd().args(&["-r"]).arg(non_utf8_dir_name).succeeds();

    assert!(!at.dir_exists(non_utf8_dir_name));
}

#[test]
#[cfg(target_os = "linux")]
fn test_rm_recursive_long_path_safe_traversal() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut deep_path = String::from("rm_deep");
    at.mkdir(&deep_path);

    for i in 0..12 {
        let long_dir_name = format!("{}{i}", "z".repeat(80));
        deep_path = format!("{deep_path}/{long_dir_name}");
        at.mkdir_all(&deep_path);
    }

    at.write("rm_deep/test1.txt", "content1");
    at.write(&format!("{deep_path}/test2.txt"), "content2");

    ts.ucmd().arg("-rf").arg("rm_deep").succeeds();

    // Verify the directory is completely removed
    assert!(!at.dir_exists("rm_deep"));
}

#[cfg(all(not(windows), feature = "chmod"))]
#[test]
fn test_rm_directory_not_executable() {
    // Test from GNU rm/rm2.sh
    // Exercise code paths when directories have no execute permission
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create directory structure: a/0, a/1/2, a/2, a/3, b/3
    at.mkdir_all("a/0");
    at.mkdir_all("a/1/2");
    at.mkdir("a/2");
    at.mkdir("a/3");
    at.mkdir_all("b/3");

    // Remove execute permission from a/1 and b
    scene.ccmd("chmod").arg("u-x").arg("a/1").succeeds();
    scene.ccmd("chmod").arg("u-x").arg("b").succeeds();

    // Try to remove both directories recursively - this should fail
    let result = scene.ucmd().args(&["-rf", "a", "b"]).fails();

    // Check for expected error messages
    // When directories don't have execute permission, we get "Permission denied"
    // when trying to access subdirectories
    let stderr = result.stderr_str();
    assert!(stderr.contains("rm: cannot remove 'a/1/2': Permission denied"));
    assert!(stderr.contains("rm: cannot remove 'b/3': Permission denied"));

    // Check which directories still exist
    assert!(!at.dir_exists("a/0")); // Should be removed
    assert!(at.dir_exists("a/1")); // Should still exist (no execute permission)
    assert!(!at.dir_exists("a/2")); // Should be removed
    assert!(!at.dir_exists("a/3")); // Should be removed

    // Restore execute permission to check b/3
    scene.ccmd("chmod").arg("u+x").arg("b").succeeds();
    assert!(at.dir_exists("b/3")); // Should still exist
}

#[cfg(all(not(windows), feature = "chmod"))]
#[test]
fn test_rm_directory_not_writable() {
    // Test from GNU rm/rm1.sh
    // Exercise code paths when directories have no write permission
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create directory structure: b/a/p, b/c, b/d
    at.mkdir_all("b/a/p");
    at.mkdir("b/c");
    at.mkdir("b/d");

    // Remove write permission from b/a
    scene.ccmd("chmod").arg("ug-w").arg("b/a").succeeds();

    // Try to remove b recursively - this should fail
    let result = scene.ucmd().args(&["-rf", "b"]).fails();

    // Check for expected error message
    // When the parent directory (b/a) doesn't have write permission,
    // we get "Permission denied" when trying to remove the subdirectory.
    // The error tracking must be correct so we don't attempt to remove the parent
    // directory after child failure (which would produce extra "Directory not empty" errors).
    result.stderr_only("rm: cannot remove 'b/a/p': Permission denied\n");

    // Check which directories still exist
    assert!(at.dir_exists("b/a/p")); // Should still exist (parent not writable)
    assert!(!at.dir_exists("b/c")); // Should be removed
    assert!(!at.dir_exists("b/d")); // Should be removed
}

#[test]
fn test_progress_flag_short() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_progress_file";

    at.touch(file);

    // Test that -g flag is accepted
    ucmd.arg("-g").arg(file).succeeds();

    assert!(!at.file_exists(file));
}

#[test]
fn test_progress_flag_long() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_progress_file";

    at.touch(file);

    // Test that --progress flag is accepted
    ucmd.arg("--progress").arg(file).succeeds();

    assert!(!at.file_exists(file));
}

#[test]
fn test_progress_with_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("test_dir");
    at.touch("test_dir/file1");
    at.touch("test_dir/file2");
    at.mkdir("test_dir/subdir");
    at.touch("test_dir/subdir/file3");

    // Test progress with recursive removal
    ucmd.arg("-rg").arg("test_dir").succeeds();

    assert!(!at.dir_exists("test_dir"));
}

#[test]
fn test_progress_with_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_rm_progress_verbose_file";

    at.touch(file);

    // Test that progress and verbose work together
    ucmd.arg("-gv").arg(file).succeeds().stdout_contains(file);

    assert!(!at.file_exists(file));
}

#[test]
fn test_progress_no_output_on_error() {
    let nonexistent_file = "this_file_does_not_exist";

    // Test that progress bar is not shown when file doesn't exist
    new_ucmd!()
        .arg("--progress")
        .arg(nonexistent_file)
        .fails()
        .stderr_contains("cannot remove")
        .stderr_contains("No such file or directory");
}

#[test]
fn no_preserve_root_may_not_be_abbreviated() {
    let (at, _ucmd) = at_and_ucmd!();
    let file = "test_file_123";

    at.touch(file);

    for arg in ["--n", "--no-pre", "--no-preserve-ro"] {
        new_ucmd!()
            .arg(arg)
            .arg(file)
            .fails()
            .stderr_contains("you may not abbreviate the --no-preserve-root option");
    }

    assert!(at.file_exists(file));
}

#[cfg(unix)]
#[test]
fn test_symlink_to_readonly_no_prompt() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("foo");
    at.set_mode("foo", 0o444);
    at.symlink_file("foo", "bar");

    ucmd.arg("---presume-input-tty")
        .arg("bar")
        .succeeds()
        .no_stderr();

    assert!(!at.symlink_exists("bar"));
}

/// Test that --preserve-root properly detects symlinks pointing to root.
#[cfg(unix)]
#[test]
fn test_preserve_root_symlink_to_root() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Create a symlink pointing to the root directory
    at.symlink_dir("/", "rootlink");

    // Attempting to recursively delete through this symlink should fail
    // because it resolves to the same device/inode as "/"
    ucmd.arg("-rf")
        .arg("--preserve-root")
        .arg("rootlink/")
        .fails()
        .stderr_contains("it is dangerous to operate recursively on")
        .stderr_contains("(same as '/')");

    // The symlink itself should still exist (we didn't delete it)
    assert!(at.symlink_exists("rootlink"));
}

/// Test that --preserve-root properly detects nested symlinks pointing to root.
#[cfg(unix)]
#[test]
fn test_preserve_root_nested_symlink_to_root() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Create a symlink pointing to the root directory
    at.symlink_dir("/", "rootlink");
    // Create another symlink pointing to the first symlink
    at.symlink_dir("rootlink", "rootlink2");

    // Attempting to recursively delete through nested symlinks should also fail
    ucmd.arg("-rf")
        .arg("--preserve-root")
        .arg("rootlink2/")
        .fails()
        .stderr_contains("it is dangerous to operate recursively on")
        .stderr_contains("(same as '/')");
}

/// Test that removing the symlink itself (not the target) still works.
#[cfg(unix)]
#[test]
fn test_preserve_root_symlink_removal_without_trailing_slash() {
    let (at, mut ucmd) = at_and_ucmd!();

    // Create a symlink pointing to the root directory
    at.symlink_dir("/", "rootlink");

    // Removing the symlink itself (without trailing slash) should succeed
    // because we're removing the link, not traversing through it
    ucmd.arg("--preserve-root").arg("rootlink").succeeds();

    assert!(!at.symlink_exists("rootlink"));
}

/// Test that literal "/" is still properly protected.
#[test]
fn test_preserve_root_literal_root() {
    new_ucmd!()
        .arg("-rf")
        .arg("--preserve-root")
        .arg("/")
        .fails()
        .stderr_contains("it is dangerous to operate recursively on '/'")
        .stderr_contains("use --no-preserve-root to override this failsafe");
}

/// Test that `rm -f` silently ignores paths that cannot be stat'd because a
/// component of the path is not a directory (ENOTDIR), matching GNU behavior.
#[cfg(unix)]
#[test]
fn test_rm_force_ignores_symlink_metadata_error() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("existing_file");
    // "existing_file/subpath" triggers ENOTDIR; -f must suppress it silently.
    ucmd.args(&["-f", "existing_file/subpath"])
        .succeeds()
        .no_stderr();
}

/// Test that without `-f`, a path that cannot be stat'd due to ENOTDIR still
/// causes a non-zero exit and reports an error.
#[cfg(unix)]
#[test]
fn test_rm_reports_error_for_symlink_metadata_failure() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("existing_file");
    ucmd.args(&["existing_file/subpath"])
        .fails()
        .stderr_contains("existing_file/subpath");
}
