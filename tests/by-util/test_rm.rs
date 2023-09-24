// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::process::Stdio;

use crate::common::util::TestScenario;

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
        "cannot remove '{file}': No such file or directory"
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
fn test_rm_force() {
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
fn test_rm_force_multiple() {
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
        .stdout_only(format!("removed directory '{dir}'\n"));

    assert!(!at.dir_exists(dir));
}

#[test]
fn test_rm_non_empty_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_non_empty_dir";
    let file_a = &format!("{dir}/test_rm_non_empty_file_a");

    at.mkdir(dir);
    at.touch(file_a);

    ucmd.arg("-d")
        .arg(dir)
        .fails()
        .stderr_contains(&format!("cannot remove '{dir}': Directory not empty"));
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
        .stderr_contains(&format!("cannot remove '{dir}': Is a directory"));
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
        .stdout_only(format!("removed '{file_a}'\nremoved '{file_b}'\n"));
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
        .stderr_contains(&format!("cannot remove '{link}': Is a directory"));

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
    ts.ucmd().fails().usage_error("missing operand");
}

#[test]
fn test_rm_verbose_slash() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_rm_verbose_slash_directory";
    let file_a = &format!("{dir}/test_rm_verbose_slash_file_a");

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
        .arg(&format!("{dir}///"))
        .succeeds()
        .stdout_only(format!(
            "removed '{file_a_normalized}'\nremoved directory '{dir}'\n"
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
fn test_rm_interactive_missing_value() {
    // `--interactive` is equivalent to `--interactive=always` or `-i`
    let (at, mut ucmd) = at_and_ucmd!();

    let file1 = "test_rm_interactive_missing_value_file1";
    let file2 = "test_rm_interactive_missing_value_file2";

    at.touch(file1);
    at.touch(file2);

    ucmd.arg("--interactive")
        .arg(file1)
        .arg(file2)
        .pipe_in("y\ny")
        .succeeds();

    assert!(!at.file_exists(file1));
    assert!(!at.file_exists(file2));
}

#[test]
fn test_rm_interactive_once_prompt() {
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
fn test_rm_interactive_once_recursive_prompt() {
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
fn test_rm_descend_directory() {
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
fn test_rm_prompts() {
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

#[test]
fn test_rm_force_prompts_order() {
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
#[cfg(any(unix, target_os = "wasi"))]
#[cfg(not(target_os = "macos"))]
fn test_non_utf8() {
    use std::ffi::OsStr;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

    let file = OsStr::from_bytes(b"not\xffutf8"); // spell-checker:disable-line

    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(file);
    assert!(at.file_exists(file));

    ucmd.arg(file).succeeds();
    assert!(!at.file_exists(file));
}
