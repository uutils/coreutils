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
fn test_ls_columns() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-columns-1"));
    at.touch(&at.plus_as_string("test-columns-2"));

    // Columns is the default
    let result = scene.ucmd().run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);

    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-columns-1\ntest-columns-2\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-columns-1  test-columns-2\n");

    for option in &["-C", "--format=columns"] {
        let result = scene.ucmd().arg(option).run();
        println!("stderr = {:?}", result.stderr);
        println!("stdout = {:?}", result.stdout);
        assert!(result.success);
        #[cfg(not(windows))]
        assert_eq!(result.stdout, "test-columns-1\ntest-columns-2\n");
        #[cfg(windows)]
        assert_eq!(result.stdout, "test-columns-1  test-columns-2\n");
    }
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

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-long"));

    for arg in &["-l", "--long", "--format=long", "--format=verbose"] {
        let result = scene.ucmd().arg(arg).arg("test-long").succeeds();
        println!("stderr = {:?}", result.stderr);
        println!("stdout = {:?}", result.stdout);
        #[cfg(not(windows))]
        assert!(result.stdout.contains("-rw-rw-r--"));

        #[cfg(windows)]
        assert!(result.stdout.contains("---------- 1 somebody somegroup"));
    }

    #[cfg(not(windows))]
    {
        unsafe {
            umask(last);
        }
    }
}

#[test]
fn test_ls_oneline() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-oneline-1"));
    at.touch(&at.plus_as_string("test-oneline-2"));

    // Bit of a weird situation: in the tests oneline and columns have the same output,
    // except on Windows.
    for option in &["-1", "--format=single-column"] {
        let result = scene.ucmd().arg(option).run();
        println!("stderr = {:?}", result.stderr);
        println!("stdout = {:?}", result.stdout);
        assert!(result.success);
        assert_eq!(result.stdout, "test-oneline-1\ntest-oneline-2\n");
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
    assert_eq!(result.stdout, "test-1\ntest-2\ntest-3\ntest-4\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-1  test-2  test-3  test-4\n");
}

#[test]
fn test_ls_long_ctime() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-long-ctime-1");
    let result = scene.ucmd().arg("-lc").succeeds();

    // Should show the time on Unix, but question marks on windows.
    #[cfg(unix)]
    assert!(result.stdout.contains(":"));
    #[cfg(not(unix))]
    assert!(result.stdout.contains("???"));
}

#[test]
fn test_ls_order_time() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-1");
    at.append("test-1", "1");
    sleep(Duration::from_millis(100));
    at.touch("test-2");
    at.append("test-2", "22");
    sleep(Duration::from_millis(100));
    at.touch("test-3");
    at.append("test-3", "333");
    sleep(Duration::from_millis(100));
    at.touch("test-4");
    at.append("test-4", "4444");
    sleep(Duration::from_millis(100));

    // Read test-3, only changing access time
    at.read("test-3");

    // Set permissions of test-2, only changing ctime
    std::fs::set_permissions(
        at.plus_as_string("test-2"),
        at.metadata("test-2").permissions(),
    )
    .unwrap();

    let result = scene.ucmd().arg("-al").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);

    // ctime was changed at write, so the order is 4 3 2 1
    let result = scene.ucmd().arg("-t").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-4\ntest-3\ntest-2\ntest-1\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-4  test-3  test-2  test-1\n");

    let result = scene.ucmd().arg("-tr").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-1\ntest-2\ntest-3\ntest-4\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-1  test-2  test-3  test-4\n");

    // 3 was accessed last in the read
    // So the order should be 2 3 4 1
    let result = scene.ucmd().arg("-tu").run();
    println!("stderr = {:?}", result.stderr);
    println!("stdout = {:?}", result.stdout);
    assert!(result.success);
    #[cfg(not(windows))]
    assert_eq!(result.stdout, "test-3\ntest-4\ntest-2\ntest-1\n");
    #[cfg(windows)]
    assert_eq!(result.stdout, "test-3  test-4  test-2  test-1\n");

    // test-2 had the last ctime change when the permissions were set
    // So the order should be 2 4 3 1
    #[cfg(unix)]
    {
        let result = scene.ucmd().arg("-tc").run();
        println!("stderr = {:?}", result.stderr);
        println!("stdout = {:?}", result.stdout);
        assert!(result.success);
        assert_eq!(result.stdout, "test-2\ntest-4\ntest-3\ntest-1\n");
    }
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

#[cfg(not(any(target_os = "macos", target_os = "windows")))] // Truncate not available on mac or win
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
    scene
        .cmd("attrib")
        .arg("+h")
        .arg("+S")
        .arg("+r")
        .arg(file)
        .run();
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
