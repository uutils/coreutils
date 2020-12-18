use crate::common::util::*;

extern crate regex;
use self::regex::Regex;

use std::thread::sleep;
use std::time::Duration;

#[cfg(not(windows))]
extern crate libc;
#[cfg(not(windows))]
use self::libc::umask;
#[cfg(not(windows))]
use std::sync::Mutex;

#[cfg(not(windows))]
lazy_static! {
    static ref UMASK_MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_ls_ls() {
    new_ucmd!().succeeds();
}

#[test]
fn test_ls_i() {
    new_ucmd!().arg("-i").succeeds();
    new_ucmd!().arg("-il").succeeds();
}

#[test]
fn test_ls_a() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(".test-1");

    let result = scene.ucmd().run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(!result.stdout.contains(".test-1"));
    assert!(!result.stdout.contains(".."));

    let result = scene.ucmd().arg("-a").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(result.stdout.contains(".test-1"));
    assert!(result.stdout.contains(".."));

    let result = scene.ucmd().arg("-A").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(result.stdout.contains(".test-1"));
    assert!(!result.stdout.contains(".."));
}

#[test]
fn test_ls_long() {
    #[cfg(not(windows))]
    let last;
    #[cfg(not(windows))]
    {
        let _guard = UMASK_MUTEX.lock();
        last = unsafe { umask(0) };

        unsafe {
            umask(0o002);
        }
    }

    let (at, mut ucmd) = at_and_ucmd!();
    at.touch(&at.plus_as_string("test-long"));
    let result = ucmd.arg("-l").arg("test-long").succeeds();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    #[cfg(not(windows))]
    assert!(result.stdout.contains("-rw-rw-r--"));

    #[cfg(windows)]
    assert!(result.stdout.contains("---------- 1 somebody somegroup"));

    #[cfg(not(windows))]
    {
        unsafe {
            umask(last);
        }
    }
}

#[test]
fn test_ls_deref() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let path_regexp = r"(.*)test-long.link -> (.*)test-long(.*)";
    let re = Regex::new(path_regexp).unwrap();

    at.touch(&at.plus_as_string("test-long"));
    at.symlink_file("test-long", "test-long.link");
    assert!(at.is_symlink("test-long.link"));

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--color=never")
        .arg("test-long")
        .arg("test-long.link")
        .run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(re.is_match(&result.stdout.trim()));

    let result = scene
        .ucmd()
        .arg("-L")
        .arg("--color=never")
        .arg("test-long")
        .arg("test-long.link")
        .run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(!re.is_match(&result.stdout.trim()));
}

#[test]
fn test_ls_order_size() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-1");
    at.append("test-1", "1");

    at.touch("test-2");
    at.append("test-2", "22");
    at.touch("test-3");
    at.append("test-3", "333");
    at.touch("test-4");
    at.append("test-4", "4444");

    let result = scene.ucmd().arg("-al").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);

    let result = scene.ucmd().arg("-S").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-4\ntest-3\ntest-2\ntest-1\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-4  test-3  test-2  test-1\n");

    let result = scene.ucmd().arg("-S").arg("-r").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-1\ntest-2\ntest-3\ntest-4\tp");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-1  test-2  test-3  test-4\n");
}

#[test]
fn test_ls_order_creation() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-1");
    at.append("test-1", "1");
    sleep(Duration::from_millis(500));
    at.touch("test-2");
    at.append("test-2", "22");
    sleep(Duration::from_millis(500));
    at.touch("test-3");
    at.append("test-3", "333");
    sleep(Duration::from_millis(500));
    at.touch("test-4");
    at.append("test-4", "4444");

    let result = scene.ucmd().arg("-al").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);

    let result = scene.ucmd().arg("-t").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-4\ntest-3\ntest-2\ntest-1\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-4  test-3  test-2  test-1\n");

    let result = scene.ucmd().arg("-t").arg("-r").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-1\ntest-2\ntest-3\ntest-4\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-1  test-2  test-3  test-4\n");
}

#[test]
fn test_ls_non_existing() {
    new_ucmd!().arg("doesntexist").fails();
}

#[test]
fn test_ls_files_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/a"));
    at.touch(&at.plus_as_string("a/b/b"));

    scene.ucmd().arg("a").succeeds();
    scene.ucmd().arg("a/a").succeeds();
    scene.ucmd().arg("a").arg("z").succeeds();

    let result = scene.ucmd().arg("doesntexist").fails();
    // Doesn't exist
    assert!(result
        .stderr
        .contains("error: 'doesntexist': No such file or directory"));

    let result = scene.ucmd().arg("a").arg("doesntexist").fails();
    // One exists, the other doesn't
    assert!(result
        .stderr
        .contains("error: 'doesntexist': No such file or directory"));
    assert!(result.stdout.contains("a:"));
}

#[test]
fn test_ls_recursive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/a"));
    at.touch(&at.plus_as_string("a/b/b"));

    scene.ucmd().arg("a").succeeds();
    scene.ucmd().arg("a/a").succeeds();
    let result = scene
        .ucmd()
        .arg("--color=never")
        .arg("-R")
        .arg("a")
        .arg("z")
        .succeeds();

    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert!(result.stdout.contains("a/b:\nb"));
    #[cfg(windows)]
    assert!(result.stdout.contains("a\\b:\nb"));
}

#[test]
fn test_ls_ls_color() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/a"));
    scene.ucmd().arg("--color").succeeds();
    scene.ucmd().arg("--color=always").succeeds();
    scene.ucmd().arg("--color=never").succeeds();
    scene.ucmd().arg("--color").arg("a").succeeds();
    scene.ucmd().arg("--color=always").arg("a/a").succeeds();
    scene.ucmd().arg("--color=never").arg("z").succeeds();
}

#[cfg(not(target_os = "macos"))] // Truncate not available on mac
#[test]
fn test_ls_human() {
    let scene = TestScenario::new(util_name!());
    let file = "test_human";
    let result = scene.cmd("truncate").arg("-s").arg("+1000").arg(file).run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    let result = scene.ucmd().arg("-hl").arg(file).run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(result.stdout.contains("1.00K"));
    scene
        .cmd("truncate")
        .arg("-s")
        .arg("+1000k")
        .arg(file)
        .run();
    let result = scene.ucmd().arg("-hl").arg(file).run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(result.stdout.contains("1.02M"));
}

#[cfg(windows)]
#[test]
fn test_ls_hidden_windows() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "hiddenWindowsFileNoDot";
    at.touch(file);
    // hide the file
    scene.cmd("attrib").arg("+h").arg("+S").arg("+r").arg(file).run();
    let result = scene.ucmd().run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    let result = scene.ucmd().arg("-a").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    assert!(result.stdout.contains(file));
}
