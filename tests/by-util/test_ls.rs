#[cfg(unix)]
extern crate unix_socket;
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
use std::path::PathBuf;
#[cfg(not(windows))]
use std::sync::Mutex;
#[cfg(not(windows))]
extern crate tempdir;
#[cfg(not(windows))]
use self::tempdir::TempDir;

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

    let result = scene.ucmd().succeeds();
    let stdout = result.stdout_str();
    assert!(!stdout.contains(".test-1"));
    assert!(!stdout.contains(".."));

    scene
        .ucmd()
        .arg("-a")
        .succeeds()
        .stdout_contains(&".test-1")
        .stdout_contains(&"..");

    let result = scene.ucmd().arg("-A").succeeds();
    result.stdout_contains(".test-1");
    let stdout = result.stdout_str();
    assert!(!stdout.contains(".."));
}

#[test]
fn test_ls_width() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-width-1"));
    at.touch(&at.plus_as_string("test-width-2"));
    at.touch(&at.plus_as_string("test-width-3"));
    at.touch(&at.plus_as_string("test-width-4"));

    for option in &["-w 100", "-w=100", "--width=100", "--width 100"] {
        scene
            .ucmd()
            .args(&option.split(" ").collect::<Vec<_>>())
            .succeeds()
            .stdout_only("test-width-1  test-width-2  test-width-3  test-width-4\n");
    }

    for option in &["-w 50", "-w=50", "--width=50", "--width 50"] {
        scene
            .ucmd()
            .args(&option.split(" ").collect::<Vec<_>>())
            .succeeds()
            .stdout_only("test-width-1  test-width-3\ntest-width-2  test-width-4\n");
    }

    for option in &[
        "-w 25",
        "-w=25",
        "--width=25",
        "--width 25",
        "-w 0",
        "-w=0",
        "--width=0",
        "--width 0",
    ] {
        scene
            .ucmd()
            .args(&option.split(" ").collect::<Vec<_>>())
            .succeeds()
            .stdout_only("test-width-1\ntest-width-2\ntest-width-3\ntest-width-4\n");
    }
}

#[test]
fn test_ls_columns() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-columns-1"));
    at.touch(&at.plus_as_string("test-columns-2"));
    at.touch(&at.plus_as_string("test-columns-3"));
    at.touch(&at.plus_as_string("test-columns-4"));

    // Columns is the default
    let result = scene.ucmd().succeeds();

    #[cfg(not(windows))]
    result.stdout_only("test-columns-1\ntest-columns-2\ntest-columns-3\ntest-columns-4\n");
    #[cfg(windows)]
    result.stdout_only("test-columns-1  test-columns-2  test-columns-3  test-columns-4\n");

    for option in &["-C", "--format=columns"] {
        let result = scene.ucmd().arg(option).succeeds();
        #[cfg(not(windows))]
        result.stdout_only("test-columns-1\ntest-columns-2\ntest-columns-3\ntest-columns-4\n");
        #[cfg(windows)]
        result.stdout_only("test-columns-1  test-columns-2  test-columns-3  test-columns-4\n");
    }

    for option in &["-C", "--format=columns"] {
        scene
            .ucmd()
            .arg("-w=40")
            .arg(option)
            .succeeds()
            .stdout_only("test-columns-1  test-columns-3\ntest-columns-2  test-columns-4\n");
    }
}

#[test]
fn test_ls_across() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-across-1"));
    at.touch(&at.plus_as_string("test-across-2"));
    at.touch(&at.plus_as_string("test-across-3"));
    at.touch(&at.plus_as_string("test-across-4"));

    for option in &["-x", "--format=across"] {
        let result = scene.ucmd().arg(option).succeeds();
        // Because the test terminal has width 0, this is the same output as
        // the columns option.
        if cfg!(unix) {
            result.stdout_only("test-across-1\ntest-across-2\ntest-across-3\ntest-across-4\n");
        } else {
            result.stdout_only("test-across-1  test-across-2  test-across-3  test-across-4\n");
        }
    }

    for option in &["-x", "--format=across"] {
        // Because the test terminal has width 0, this is the same output as
        // the columns option.
        scene
            .ucmd()
            .arg("-w=30")
            .arg(option)
            .succeeds()
            .stdout_only("test-across-1  test-across-2\ntest-across-3  test-across-4\n");
    }
}

#[test]
fn test_ls_commas() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-commas-1"));
    at.touch(&at.plus_as_string("test-commas-2"));
    at.touch(&at.plus_as_string("test-commas-3"));
    at.touch(&at.plus_as_string("test-commas-4"));

    for option in &["-m", "--format=commas"] {
        let result = scene.ucmd().arg(option).succeeds();
        if cfg!(unix) {
            result.stdout_only("test-commas-1,\ntest-commas-2,\ntest-commas-3,\ntest-commas-4\n");
        } else {
            result.stdout_only("test-commas-1, test-commas-2, test-commas-3, test-commas-4\n");
        }
    }

    for option in &["-m", "--format=commas"] {
        scene
            .ucmd()
            .arg("-w=30")
            .arg(option)
            .succeeds()
            .stdout_only("test-commas-1, test-commas-2,\ntest-commas-3, test-commas-4\n");
    }
    for option in &["-m", "--format=commas"] {
        scene
            .ucmd()
            .arg("-w=45")
            .arg(option)
            .succeeds()
            .stdout_only("test-commas-1, test-commas-2, test-commas-3,\ntest-commas-4\n");
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
        #[cfg(not(windows))]
        result.stdout_contains("-rw-rw-r--");

        #[cfg(windows)]
        result.stdout_contains("---------- 1 somebody somegroup");
    }

    #[cfg(not(windows))]
    {
        unsafe {
            umask(last);
        }
    }
}

#[test]
fn test_ls_long_formats() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(&at.plus_as_string("test-long-formats"));

    // Regex for three names, so all of author, group and owner
    let re_three = Regex::new(r"[xrw-]{9} \d ([-0-9_a-z]+ ){3}0").unwrap();

    #[cfg(unix)]
    let re_three_num = Regex::new(r"[xrw-]{9} \d (\d+ ){3}0").unwrap();

    // Regex for two names, either:
    // - group and owner
    // - author and owner
    // - author and group
    let re_two = Regex::new(r"[xrw-]{9} \d ([-0-9_a-z]+ ){2}0").unwrap();

    #[cfg(unix)]
    let re_two_num = Regex::new(r"[xrw-]{9} \d (\d+ ){2}0").unwrap();

    // Regex for one name: author, group or owner
    let re_one = Regex::new(r"[xrw-]{9} \d [-0-9_a-z]+ 0").unwrap();

    #[cfg(unix)]
    let re_one_num = Regex::new(r"[xrw-]{9} \d \d+ 0").unwrap();

    // Regex for no names
    let re_zero = Regex::new(r"[xrw-]{9} \d 0").unwrap();

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--author")
        .arg("test-long-formats")
        .succeeds();
    assert!(re_three.is_match(result.stdout_str()));

    let result = scene
        .ucmd()
        .arg("-l1")
        .arg("--author")
        .arg("test-long-formats")
        .succeeds();
    assert!(re_three.is_match(&result.stdout_str()));

    #[cfg(unix)]
    {
        let result = scene
            .ucmd()
            .arg("-n")
            .arg("--author")
            .arg("test-long-formats")
            .succeeds();
        assert!(re_three_num.is_match(result.stdout_str()));
    }

    for arg in &[
        "-l",                     // only group and owner
        "-g --author",            // only author and group
        "-o --author",            // only author and owner
        "-lG --author",           // only author and owner
        "-l --no-group --author", // only author and owner
    ] {
        let result = scene
            .ucmd()
            .args(&arg.split(" ").collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds();
        assert!(re_two.is_match(result.stdout_str()));

        #[cfg(unix)]
        {
            let result = scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(" ").collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds();
            assert!(re_two_num.is_match(result.stdout_str()));
        }
    }

    for arg in &[
        "-g",            // only group
        "-gl",           // only group
        "-o",            // only owner
        "-ol",           // only owner
        "-oG",           // only owner
        "-lG",           // only owner
        "-l --no-group", // only owner
        "-gG --author",  // only author
    ] {
        let result = scene
            .ucmd()
            .args(&arg.split(" ").collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds();
        assert!(re_one.is_match(result.stdout_str()));

        #[cfg(unix)]
        {
            let result = scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(" ").collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds();
            assert!(re_one_num.is_match(result.stdout_str()));
        }
    }

    for arg in &[
        "-og",
        "-ogl",
        "-lgo",
        "-gG",
        "-g --no-group",
        "-og --no-group",
        "-og --format=long",
        "-ogCl",
        "-og --format=vertical -l",
        "-og1",
        "-og1l",
    ] {
        let result = scene
            .ucmd()
            .args(&arg.split(" ").collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds();
        assert!(re_zero.is_match(result.stdout_str()));

        #[cfg(unix)]
        {
            let result = scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(" ").collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds();
            assert!(re_zero.is_match(result.stdout_str()));
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
        scene
            .ucmd()
            .arg(option)
            .succeeds()
            .stdout_only("test-oneline-1\ntest-oneline-2\n");
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
        .succeeds();
    assert!(re.is_match(result.stdout_str().trim()));

    let result = scene
        .ucmd()
        .arg("-L")
        .arg("--color=never")
        .arg("test-long")
        .arg("test-long.link")
        .succeeds();
    assert!(!re.is_match(result.stdout_str().trim()));
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

    scene.ucmd().arg("-al").succeeds();

    let result = scene.ucmd().arg("-S").succeeds();
    #[cfg(not(windows))]
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");
    #[cfg(windows)]
    result.stdout_only("test-4  test-3  test-2  test-1\n");

    let result = scene.ucmd().arg("-S").arg("-r").succeeds();
    #[cfg(not(windows))]
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");
    #[cfg(windows)]
    result.stdout_only("test-1  test-2  test-3  test-4\n");
}

#[test]
fn test_ls_long_ctime() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-long-ctime-1");
    let result = scene.ucmd().arg("-lc").succeeds();

    // Should show the time on Unix, but question marks on windows.
    #[cfg(unix)]
    result.stdout_contains(":");
    #[cfg(not(unix))]
    result.stdout_contains("???");
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

    scene.ucmd().arg("-al").succeeds();

    // ctime was changed at write, so the order is 4 3 2 1
    let result = scene.ucmd().arg("-t").succeeds();
    #[cfg(not(windows))]
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");
    #[cfg(windows)]
    result.stdout_only("test-4  test-3  test-2  test-1\n");

    let result = scene.ucmd().arg("-tr").succeeds();
    #[cfg(not(windows))]
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");
    #[cfg(windows)]
    result.stdout_only("test-1  test-2  test-3  test-4\n");

    // 3 was accessed last in the read
    // So the order should be 2 3 4 1
    let result = scene.ucmd().arg("-tu").succeeds();
    let file3_access = at.open("test-3").metadata().unwrap().accessed().unwrap();
    let file4_access = at.open("test-4").metadata().unwrap().accessed().unwrap();

    // It seems to be dependent on the platform whether the access time is actually set
    if file3_access > file4_access {
        if cfg!(not(windows)) {
            result.stdout_only("test-3\ntest-4\ntest-2\ntest-1\n");
        } else {
            result.stdout_only("test-3  test-4  test-2  test-1\n");
        }
    } else {
        // Access time does not seem to be set on Windows and some other
        // systems so the order is 4 3 2 1
        if cfg!(not(windows)) {
            result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");
        } else {
            result.stdout_only("test-4  test-3  test-2  test-1\n");
        }
    }

    // test-2 had the last ctime change when the permissions were set
    // So the order should be 2 4 3 1
    #[cfg(unix)]
    {
        let result = scene.ucmd().arg("-tc").succeeds();
        result.stdout_only("test-2\ntest-4\ntest-3\ntest-1\n");
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

    // Doesn't exist
    scene
        .ucmd()
        .arg("doesntexist")
        .fails()
        .stderr_contains(&"error: 'doesntexist': No such file or directory");

    // One exists, the other doesn't
    scene
        .ucmd()
        .arg("a")
        .arg("doesntexist")
        .fails()
        .stderr_contains(&"error: 'doesntexist': No such file or directory")
        .stdout_contains(&"a:");
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

    #[cfg(not(windows))]
    result.stdout_contains(&"a/b:\nb");
    #[cfg(windows)]
    result.stdout_contains(&"a\\b:\nb");
}

#[cfg(unix)]
#[test]
fn test_ls_ls_color() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("a/nested_dir");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/nested_file"));
    at.touch("test-color");

    let a_with_colors = "\x1b[01;34ma\x1b[0m";
    let z_with_colors = "\x1b[01;34mz\x1b[0m";
    let nested_dir_with_colors = "\x1b[01;34mnested_dir\x1b[0m";

    // Color is disabled by default
    let result = scene.ucmd().succeeds();
    assert!(!result.stdout_str().contains(a_with_colors));
    assert!(!result.stdout_str().contains(z_with_colors));

    // Color should be enabled
    scene
        .ucmd()
        .arg("--color")
        .succeeds()
        .stdout_contains(a_with_colors)
        .stdout_contains(z_with_colors);

    // Color should be enabled
    scene
        .ucmd()
        .arg("--color=always")
        .succeeds()
        .stdout_contains(a_with_colors)
        .stdout_contains(z_with_colors);

    // Color should be disabled
    let result = scene.ucmd().arg("--color=never").succeeds();
    assert!(!result.stdout_str().contains(a_with_colors));
    assert!(!result.stdout_str().contains(z_with_colors));

    // Nested dir should be shown and colored
    scene
        .ucmd()
        .arg("--color")
        .arg("a")
        .succeeds()
        .stdout_contains(nested_dir_with_colors);

    // Color has no effect
    scene
        .ucmd()
        .arg("--color=always")
        .arg("a/nested_file")
        .succeeds()
        .stdout_contains("a/nested_file\n");

    // No output
    scene
        .ucmd()
        .arg("--color=never")
        .arg("z")
        .succeeds()
        .stdout_only("");
}

#[cfg(unix)]
#[test]
fn test_ls_inode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_inode";
    at.touch(file);

    let re_short = Regex::new(r" *(\d+) test_inode").unwrap();
    let re_long = Regex::new(r" *(\d+) [xrw-]{10} \d .+ test_inode").unwrap();

    let result = scene.ucmd().arg("test_inode").arg("-i").succeeds();
    assert!(re_short.is_match(result.stdout_str()));
    let inode_short = re_short
        .captures(result.stdout_str())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    let result = scene.ucmd().arg("test_inode").succeeds();
    assert!(!re_short.is_match(result.stdout_str()));
    assert!(!result.stdout_str().contains(inode_short));

    let result = scene.ucmd().arg("-li").arg("test_inode").succeeds();
    assert!(re_long.is_match(result.stdout_str()));
    let inode_long = re_long
        .captures(result.stdout_str())
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();

    let result = scene.ucmd().arg("-l").arg("test_inode").succeeds();
    assert!(!re_long.is_match(result.stdout_str()));
    assert!(!result.stdout_str().contains(inode_long));

    assert_eq!(inode_short, inode_long)
}

#[test]
#[cfg(not(windows))]
fn test_ls_indicator_style() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Setup: Directory, Symlink, and Pipes.
    at.mkdir("directory");
    assert!(at.dir_exists("directory"));

    at.touch(&at.plus_as_string("link-src"));
    at.symlink_file("link-src", "link-dest.link");
    assert!(at.is_symlink("link-dest.link"));

    at.mkfifo("named-pipe.fifo");
    assert!(at.is_fifo("named-pipe.fifo"));

    // Classify, File-Type, and Slash all contain indicators for directories.
    let options = vec!["classify", "file-type", "slash"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene
            .ucmd()
            .arg(format!("--indicator-style={}", opt))
            .succeeds()
            .stdout_contains(&"/");
    }

    // Same test as above, but with the alternate flags.
    let options = vec!["--classify", "--file-type", "-p"];
    for opt in options {
        scene
            .ucmd()
            .arg(format!("{}", opt))
            .succeeds()
            .stdout_contains(&"/");
    }

    // Classify and File-Type all contain indicators for pipes and links.
    let options = vec!["classify", "file-type"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene
            .ucmd()
            .arg(format!("--indicator-style={}", opt))
            .succeeds()
            .stdout_contains(&"@")
            .stdout_contains(&"|");
    }

    // Test sockets. Because the canonical way of making sockets to test is with
    // TempDir, we need a separate test.
    {
        use self::unix_socket::UnixListener;

        let dir = TempDir::new("unix_socket").expect("failed to create dir");
        let socket_path = dir.path().join("sock");
        let _listener = UnixListener::bind(&socket_path).expect("failed to create socket");

        new_ucmd!()
            .args(&[
                PathBuf::from(dir.path().to_str().unwrap()),
                PathBuf::from("--indicator-style=classify"),
            ])
            .succeeds()
            .stdout_only("sock=\n");
    }
}

// Essentially the same test as above, but only test symlinks and directories,
// not pipes or sockets.
#[test]
#[cfg(not(unix))]
fn test_ls_indicator_style() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Setup: Directory, Symlink.
    at.mkdir("directory");
    assert!(at.dir_exists("directory"));

    at.touch(&at.plus_as_string("link-src"));
    at.symlink_file("link-src", "link-dest.link");
    assert!(at.is_symlink("link-dest.link"));

    // Classify, File-Type, and Slash all contain indicators for directories.
    let options = vec!["classify", "file-type", "slash"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        let result = scene
            .ucmd()
            .arg(format!("--indicator-style={}", opt))
            .succeeds()
            .stdout_contains(&"/");
    }

    // Same test as above, but with the alternate flags.
    let options = vec!["--classify", "--file-type", "-p"];
    for opt in options {
        let result = scene
            .ucmd()
            .arg(format!("{}", opt))
            .succeeds()
            .stdout_contains(&"/");
    }

    // Classify and File-Type all contain indicators for pipes and links.
    let options = vec!["classify", "file-type"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        let result = scene
            .ucmd()
            .arg(format!("--indicator-style={}", opt))
            .succeeds()
            .stdout_contains(&"@");
    }
}

#[cfg(not(any(target_vendor = "apple", target_os = "windows")))] // Truncate not available on mac or win
#[test]
fn test_ls_human_si() {
    let scene = TestScenario::new(util_name!());
    let file1 = "test_human-1";
    scene
        .cmd("truncate")
        .arg("-s")
        .arg("+1000")
        .arg(file1)
        .succeeds();

    scene
        .ucmd()
        .arg("-hl")
        .arg(file1)
        .succeeds()
        .stdout_contains(" 1000 ");

    scene
        .ucmd()
        .arg("-l")
        .arg("--si")
        .arg(file1)
        .succeeds()
        .stdout_contains(" 1.0k ");

    scene
        .cmd("truncate")
        .arg("-s")
        .arg("+1000k")
        .arg(file1)
        .run();

    scene
        .ucmd()
        .arg("-hl")
        .arg(file1)
        .succeeds()
        .stdout_contains(" 1001K ");

    scene
        .ucmd()
        .arg("-l")
        .arg("--si")
        .arg(file1)
        .succeeds()
        .stdout_contains(" 1.1M ");

    let file2 = "test-human-2";
    scene
        .cmd("truncate")
        .arg("-s")
        .arg("+12300k")
        .arg(file2)
        .succeeds();

    // GNU rounds up, so we must too.
    scene
        .ucmd()
        .arg("-hl")
        .arg(file2)
        .succeeds()
        .stdout_contains(" 13M ");

    // GNU rounds up, so we must too.
    scene
        .ucmd()
        .arg("-l")
        .arg("--si")
        .arg(file2)
        .succeeds()
        .stdout_contains(" 13M ");

    let file3 = "test-human-3";
    scene
        .cmd("truncate")
        .arg("-s")
        .arg("+9999")
        .arg(file3)
        .succeeds();

    scene
        .ucmd()
        .arg("-hl")
        .arg(file3)
        .succeeds()
        .stdout_contains(" 9.8K ");

    scene
        .ucmd()
        .arg("-l")
        .arg("--si")
        .arg(file3)
        .succeeds()
        .stdout_contains(" 10k ");
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
        .succeeds();

    let result = scene.ucmd().succeeds();
    assert!(!result.stdout_str().contains(file));
    let result = scene.ucmd().arg("-a").succeeds().stdout_contains(file);
}

#[test]
fn test_ls_version_sort() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    for filename in &[
        "a2",
        "b1",
        "b20",
        "a1.4",
        "a1.40",
        "b3",
        "b11",
        "b20b",
        "b20a",
        "a100",
        "a1.13",
        "aa",
        "a1",
        "aaa",
        "a1.00000040",
        "abab",
        "ab",
        "a01.40",
        "a001.001",
        "a01.0000001",
        "a01.001",
        "a001.01",
    ] {
        at.touch(filename);
    }

    let mut expected = vec![
        "a1",
        "a001.001",
        "a001.01",
        "a01.0000001",
        "a01.001",
        "a1.4",
        "a1.13",
        "a01.40",
        "a1.00000040",
        "a1.40",
        "a2",
        "a100",
        "aa",
        "aaa",
        "ab",
        "abab",
        "b1",
        "b3",
        "b11",
        "b20",
        "b20a",
        "b20b",
        "", // because of '\n' at the end of the output
    ];

    let result = scene.ucmd().arg("-1v").succeeds();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected
    );

    let result = scene.ucmd().arg("-1").arg("--sort=version").succeeds();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected
    );

    let result = scene.ucmd().arg("-a1v").succeeds();
    expected.insert(0, "..");
    expected.insert(0, ".");
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected,
    )
}

#[test]
fn test_ls_quoting_style() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("one two");
    at.touch("one");

    // It seems that windows doesn't allow \n in filenames.
    #[cfg(unix)]
    {
        at.touch("one\ntwo");
        // Default is shell-escape
        scene
            .ucmd()
            .arg("one\ntwo")
            .succeeds()
            .stdout_only("'one'$'\\n''two'\n");

        for (arg, correct) in &[
            ("--quoting-style=literal", "one?two"),
            ("-N", "one?two"),
            ("--literal", "one?two"),
            ("--quoting-style=c", "\"one\\ntwo\""),
            ("-Q", "\"one\\ntwo\""),
            ("--quote-name", "\"one\\ntwo\""),
            ("--quoting-style=escape", "one\\ntwo"),
            ("-b", "one\\ntwo"),
            ("--escape", "one\\ntwo"),
            ("--quoting-style=shell-escape", "'one'$'\\n''two'"),
            ("--quoting-style=shell-escape-always", "'one'$'\\n''two'"),
            ("--quoting-style=shell", "one?two"),
            ("--quoting-style=shell-always", "'one?two'"),
        ] {
            scene
                .ucmd()
                .arg(arg)
                .arg("one\ntwo")
                .succeeds()
                .stdout_only(format!("{}\n", correct));
        }

        for (arg, correct) in &[
            ("--quoting-style=literal", "one?two"),
            ("-N", "one?two"),
            ("--literal", "one?two"),
            ("--quoting-style=shell", "one?two"),
            ("--quoting-style=shell-always", "'one?two'"),
        ] {
            scene
                .ucmd()
                .arg(arg)
                .arg("--hide-control-chars")
                .arg("one\ntwo")
                .succeeds()
                .stdout_only(format!("{}\n", correct));
        }

        for (arg, correct) in &[
            ("--quoting-style=literal", "one\ntwo"),
            ("-N", "one\ntwo"),
            ("--literal", "one\ntwo"),
            ("--quoting-style=shell", "one\ntwo"),
            ("--quoting-style=shell-always", "'one\ntwo'"),
        ] {
            scene
                .ucmd()
                .arg(arg)
                .arg("--show-control-chars")
                .arg("one\ntwo")
                .succeeds()
                .stdout_only(format!("{}\n", correct));
        }
    }

    scene
        .ucmd()
        .arg("one two")
        .succeeds()
        .stdout_only("'one two'\n");

    for (arg, correct) in &[
        ("--quoting-style=literal", "one two"),
        ("-N", "one two"),
        ("--literal", "one two"),
        ("--quoting-style=c", "\"one two\""),
        ("-Q", "\"one two\""),
        ("--quote-name", "\"one two\""),
        ("--quoting-style=escape", "one\\ two"),
        ("-b", "one\\ two"),
        ("--escape", "one\\ two"),
        ("--quoting-style=shell-escape", "'one two'"),
        ("--quoting-style=shell-escape-always", "'one two'"),
        ("--quoting-style=shell", "'one two'"),
        ("--quoting-style=shell-always", "'one two'"),
    ] {
        scene
            .ucmd()
            .arg(arg)
            .arg("one two")
            .succeeds()
            .stdout_only(format!("{}\n", correct));
    }

    scene.ucmd().arg("one").succeeds().stdout_only("one\n");

    for (arg, correct) in &[
        ("--quoting-style=literal", "one"),
        ("-N", "one"),
        ("--quoting-style=c", "\"one\""),
        ("-Q", "\"one\""),
        ("--quote-name", "\"one\""),
        ("--quoting-style=escape", "one"),
        ("-b", "one"),
        ("--quoting-style=shell-escape", "one"),
        ("--quoting-style=shell-escape-always", "'one'"),
        ("--quoting-style=shell", "one"),
        ("--quoting-style=shell-always", "'one'"),
    ] {
        scene
            .ucmd()
            .arg(arg)
            .arg("one")
            .succeeds()
            .stdout_only(format!("{}\n", correct));
    }
}

#[test]
fn test_ls_ignore_hide() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("README.md");
    at.touch("CONTRIBUTING.md");
    at.touch("some_other_file");
    at.touch("READMECAREFULLY.md");

    scene
        .ucmd()
        .arg("--hide")
        .arg("*")
        .arg("-1")
        .succeeds()
        .stdout_only("");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("*")
        .arg("-1")
        .succeeds()
        .stdout_only("");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("irrelevant pattern")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("README*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--hide")
        .arg("README*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore")
        .arg("*.md")
        .arg("-1")
        .succeeds()
        .stdout_only(".\n..\nsome_other_file\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("--hide")
        .arg("*.md")
        .arg("-1")
        .succeeds()
        .stdout_only(".\n..\nCONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("-A")
        .arg("--ignore")
        .arg("*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("-A")
        .arg("--hide")
        .arg("*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    // Stacking multiple patterns
    scene
        .ucmd()
        .arg("--ignore")
        .arg("README*")
        .arg("--ignore")
        .arg("CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("--hide")
        .arg("README*")
        .arg("--ignore")
        .arg("CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("--hide")
        .arg("README*")
        .arg("--hide")
        .arg("CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    // Invalid patterns
    scene
        .ucmd()
        .arg("--ignore")
        .arg("READ[ME")
        .arg("-1")
        .succeeds()
        .stderr_contains(&"Invalid pattern")
        .stdout_is("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--hide")
        .arg("READ[ME")
        .arg("-1")
        .succeeds()
        .stderr_contains(&"Invalid pattern")
        .stdout_is("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");
}

#[test]
fn test_ls_directory() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("some_dir");
    at.symlink_dir("some_dir", "sym_dir");

    at.touch("some_dir/nested_file");

    scene
        .ucmd()
        .arg("some_dir")
        .succeeds()
        .stdout_is("nested_file\n");

    scene
        .ucmd()
        .arg("--directory")
        .arg("some_dir")
        .succeeds()
        .stdout_is("some_dir\n");

    scene
        .ucmd()
        .arg("sym_dir")
        .succeeds()
        .stdout_is("nested_file\n");
}

#[test]
fn test_ls_deref_command_line() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("some_file");
    at.symlink_file("some_file", "sym_file");

    scene
        .ucmd()
        .arg("sym_file")
        .succeeds()
        .stdout_is("sym_file\n");

    // -l changes the default to no dereferencing
    scene
        .ucmd()
        .arg("-l")
        .arg("sym_file")
        .succeeds()
        .stdout_contains("sym_file ->");

    scene
        .ucmd()
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_file")
        .succeeds()
        .stdout_is("sym_file\n");

    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_file")
        .succeeds()
        .stdout_contains("sym_file ->");

    scene
        .ucmd()
        .arg("--dereference-command-line")
        .arg("sym_file")
        .succeeds()
        .stdout_is("sym_file\n");

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line")
        .arg("sym_file")
        .succeeds();

    assert!(!result.stdout_str().contains("->"));

    let result = scene.ucmd().arg("-lH").arg("sym_file").succeeds();

    assert!(!result.stdout_str().contains("sym_file ->"));

    // If the symlink is not a command line argument, it must be shown normally
    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line")
        .succeeds()
        .stdout_contains("sym_file ->");
}

#[test]
fn test_ls_deref_command_line_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("some_dir");
    at.symlink_dir("some_dir", "sym_dir");

    at.touch("some_dir/nested_file");

    scene
        .ucmd()
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    scene
        .ucmd()
        .arg("-l")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("sym_dir ->");

    scene
        .ucmd()
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    scene
        .ucmd()
        .arg("--dereference-command-line")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    scene
        .ucmd()
        .arg("-lH")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("nested_file");

    // If the symlink is not a command line argument, it must be shown normally
    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line")
        .succeeds()
        .stdout_contains("sym_dir ->");

    scene
        .ucmd()
        .arg("-lH")
        .succeeds()
        .stdout_contains("sym_dir ->");

    scene
        .ucmd()
        .arg("-l")
        .arg("--dereference-command-line-symlink-to-dir")
        .succeeds()
        .stdout_contains("sym_dir ->");

    // --directory does not dereference anything by default
    scene
        .ucmd()
        .arg("-l")
        .arg("--directory")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("sym_dir ->");

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--directory")
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_dir")
        .succeeds();

    assert!(!result.stdout_str().ends_with("sym_dir"));

    // --classify does not dereference anything by default
    scene
        .ucmd()
        .arg("-l")
        .arg("--directory")
        .arg("sym_dir")
        .succeeds()
        .stdout_contains("sym_dir ->");

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--directory")
        .arg("--dereference-command-line-symlink-to-dir")
        .arg("sym_dir")
        .succeeds();

    assert!(!result.stdout_str().ends_with("sym_dir"));
}
