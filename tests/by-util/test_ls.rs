// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) READMECAREFULLY birthtime doesntexist oneline somebackup lrwx somefile somegroup somehiddenbackup somehiddenfile tabsize aaaaaaaa bbbb cccc dddddddd ncccc neee naaaaa nbcdef nfffff dired subdired tmpfs mdir COLORTERM mexe bcdef

#[cfg(any(unix, feature = "feat_selinux"))]
use crate::common::util::expected_result;
use crate::common::util::TestScenario;
#[cfg(all(unix, feature = "chmod"))]
use nix::unistd::{close, dup};
use regex::Regex;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
#[cfg(all(unix, feature = "chmod"))]
use std::os::unix::io::IntoRawFd;
use std::path::Path;
#[cfg(not(windows))]
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

const LONG_ARGS: &[&str] = &[
    "-l",
    "--long",
    "--format=long",
    "--format=lon",
    "--for=long",
    "--format=verbose",
    "--for=verbose",
];

const ACROSS_ARGS: &[&str] = &[
    "-x",
    "--format=across",
    "--format=acr",
    "--format=horizontal",
    "--for=across",
    "--for=horizontal",
];

const COMMA_ARGS: &[&str] = &["-m", "--format=commas", "--for=commas"];

const COLUMN_ARGS: &[&str] = &["-C", "--format=columns", "--for=columns"];

#[test]
fn test_invalid_flag() {
    new_ucmd!()
        .arg("--invalid-argument")
        .fails()
        .no_stdout()
        .code_is(2);
}

#[test]
fn test_invalid_value_returns_1() {
    // Invalid values to flags *sometimes* result in error code 1:
    for flag in [
        "--classify",
        "--color",
        "--format",
        "--hyperlink",
        "--indicator-style",
        "--quoting-style",
        "--sort",
        "--time",
    ] {
        new_ucmd!()
            .arg(&format!("{flag}=definitely_invalid_value"))
            .fails()
            .no_stdout()
            .code_is(1);
    }
}

#[test]
fn test_invalid_value_returns_2() {
    // Invalid values to flags *sometimes* result in error code 2:
    for flag in ["--block-size", "--width", "--tab-size"] {
        new_ucmd!()
            .arg(&format!("{flag}=definitely_invalid_value"))
            .fails()
            .no_stdout()
            .code_is(2);
    }
}

#[test]
fn test_invalid_value_time_style() {
    // This is the only flag which does not raise an error if it is invalid but not actually used:
    new_ucmd!()
        .arg("--time-style=definitely_invalid_value")
        .succeeds()
        .no_stderr()
        .code_is(0);
    // If it is used, error:
    new_ucmd!()
        .arg("-g")
        .arg("--time-style=definitely_invalid_value")
        .fails()
        .no_stdout()
        .code_is(2);
    // If it only looks temporarily like it might be used, no error:
    new_ucmd!()
        .arg("-l")
        .arg("--time-style=definitely_invalid_value")
        .arg("--format=single-column")
        .succeeds()
        .no_stderr()
        .code_is(0);
}

#[test]
fn test_ls_ls() {
    new_ucmd!().succeeds();
}

#[test]
fn test_ls_help() {
    // Because we have to work around a lot of clap's error handling and
    // modify the exit-code, this is actually non-trivial.
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("--version")
        .no_stderr();
}

#[test]
fn test_ls_i() {
    new_ucmd!().arg("-i").succeeds();
    new_ucmd!().arg("-il").succeeds();
}

#[test]
fn test_ls_ordering() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.mkdir("some-dir2");
    at.mkdir("some-dir3");
    at.mkdir("some-dir4");
    at.mkdir("some-dir5");
    at.mkdir("some-dir6");

    scene
        .ucmd()
        .arg("-Rl")
        .succeeds()
        .stdout_matches(&Regex::new("some-dir1:\\ntotal 0").unwrap());
}

#[cfg(all(unix, feature = "df", not(target_os = "freebsd")))]
fn get_filesystem_type(scene: &TestScenario, path: &Path) -> String {
    let mut cmd = scene.ccmd("df");
    cmd.args(&["-PT"]).arg(path);
    let output = cmd.succeeds();
    let stdout_str = String::from_utf8_lossy(output.stdout());
    println!("output of stat call ({:?}):\n{}", cmd, stdout_str);
    let regex_str = r#"Filesystem\s+Type\s+.+[\r\n]+([^\s]+)\s+(?<fstype>[^\s]+)\s+"#;
    let regex = Regex::new(regex_str).unwrap();
    let m = regex.captures(&stdout_str).unwrap();
    let fstype = m["fstype"].to_owned();
    println!("detected fstype: {}", fstype);
    fstype
}

#[cfg(all(feature = "truncate", feature = "dd"))]
#[test] // FIXME: fix this test for FreeBSD
fn test_ls_allocation_size() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-dir1/empty-file");

    #[cfg(all(unix, feature = "df"))]
    {
        scene
            .ccmd("truncate")
            .arg("-s")
            .arg("4M")
            .arg("some-dir1/file-with-holes")
            .succeeds();

        // fill empty file with zeros
        scene
            .ccmd("dd")
            .arg("if=/dev/zero")
            .arg("of=some-dir1/zero-file")
            .arg("bs=1024")
            .arg("count=4096")
            .succeeds();

        scene
            .ccmd("dd")
            .arg("if=/dev/zero")
            .arg("of=irregular-file")
            .arg("bs=1")
            .arg("count=777")
            .succeeds();

        scene
            .ucmd()
            .arg("-l")
            .arg("--block-size=512")
            .arg("irregular-file")
            .succeeds()
            .stdout_matches(&Regex::new("[^ ] 2 [^ ]").unwrap());

        #[cfg(not(target_os = "freebsd"))]
        let (zero_file_size_4k, zero_file_size_1k, zero_file_size_8k, zero_file_size_4m) =
            match get_filesystem_type(&scene, &scene.fixtures.subdir).as_str() {
                // apparently f2fs (flash friendly fs) accepts small overhead for better performance
                "f2fs" => (4100, 1025, 8200, "4.1M"),
                _ => (4096, 1024, 8192, "4.0M"),
            };

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_is(format!(
                "total {zero_file_size_4k}\n   0 empty-file\n   0 file-with-holes\n\
                {zero_file_size_4k} zero-file\n"
            ));

        scene
            .ucmd()
            .arg("-sl")
            .arg("some-dir1")
            .succeeds()
            // block size is 0 whereas size/len is 4194304
            .stdout_contains("4194304");

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains("0 empty-file")
            .stdout_contains(format!("{zero_file_size_4k} zero-file"));

        // Test alignment of different block sized files
        let res = scene.ucmd().arg("-si1").arg("some-dir1").succeeds();

        let empty_file_len = String::from_utf8(res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .strip_suffix("empty-file")
            .unwrap()
            .len();

        let file_with_holes_len = String::from_utf8(res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .nth(2)
            .unwrap()
            .strip_suffix("file-with-holes")
            .unwrap()
            .len();

        assert_eq!(empty_file_len, file_with_holes_len);

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("LS_BLOCK_SIZE", "8K")
            .env("BLOCK_SIZE", "4K")
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains("total 512")
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains("512 zero-file");

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("BLOCK_SIZE", "4K")
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_1k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_1k} zero-file"));

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("BLOCK_SIZE", "4K")
            .arg("-s1")
            .arg("--si")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains("total 4.2M")
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains("4.2M zero-file");

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("BLOCK_SIZE", "4096")
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_1k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_1k} zero-file"));

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("POSIXLY_CORRECT", "true")
            .arg("-s1")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_8k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_8k} zero-file"));

        // -k should make 'ls' ignore the env var
        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .env("BLOCK_SIZE", "4K")
            .arg("-s1k")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_4k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_4k} zero-file"));

        // but manually specified blocksize overrides -k
        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1k")
            .arg("--block-size=4K")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_1k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_1k} zero-file"));

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1")
            .arg("--block-size=4K")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_1k}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_1k} zero-file"));

        // si option should always trump the human-readable option
        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1h")
            .arg("--si")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains("total 4.2M")
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains("4.2M zero-file");

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1")
            .arg("--block-size=human-readable")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains(format!("total {zero_file_size_4m}"))
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains(format!("{zero_file_size_4m} zero-file"));

        #[cfg(not(target_os = "freebsd"))]
        scene
            .ucmd()
            .arg("-s1")
            .arg("--block-size=si")
            .arg("some-dir1")
            .succeeds()
            .stdout_contains("total 4.2M")
            .stdout_contains("0 empty-file")
            .stdout_contains("0 file-with-holes")
            .stdout_contains("4.2M zero-file");
    }
}

#[test]
fn test_ls_devices() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");

    // Regex tests correct device ID and correct (no pad) spacing for a single file
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        scene
            .ucmd()
            .arg("-al")
            .arg("/dev/null")
            .succeeds()
            .stdout_matches(&Regex::new("[^ ] 3, 2 [^ ]").unwrap());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        scene
            .ucmd()
            .arg("-al")
            .arg("/dev/null")
            .succeeds()
            .stdout_matches(&Regex::new("[^ ] 1, 3 [^ ]").unwrap());
    }

    // Tests display alignment against a file (stdout is a link to a tty)
    #[cfg(unix)]
    {
        #[cfg(not(target_os = "android"))]
        let stdout = "/dev/stdout";
        #[cfg(target_os = "android")]
        let stdout = "/proc/self/fd/1";
        let res = scene
            .ucmd()
            .arg("-alL")
            .arg("/dev/null")
            .arg(stdout)
            .succeeds();

        let null_len = String::from_utf8(res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .strip_suffix("/dev/null")
            .unwrap()
            .len();

        let stdout_len = String::from_utf8(res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .strip_suffix(stdout)
            .unwrap()
            .len();

        assert_eq!(stdout_len, null_len);
    }
}

#[cfg(feature = "chmod")]
#[test]
fn test_ls_io_errors() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.mkdir("some-dir2");
    at.symlink_file("does_not_exist", "some-dir2/dangle");
    at.mkdir("some-dir3");
    at.mkdir("some-dir3/some-dir4");
    at.mkdir("some-dir4");

    scene.ccmd("chmod").arg("000").arg("some-dir1").succeeds();

    scene
        .ucmd()
        .arg("-1")
        .arg("some-dir1")
        .fails()
        .code_is(2)
        .stderr_contains("cannot open directory")
        .stderr_contains("Permission denied");

    scene
        .ucmd()
        .arg("-Li")
        .arg("some-dir2")
        .fails()
        .code_is(1)
        .stderr_contains("cannot access")
        .stderr_contains("No such file or directory")
        .stdout_contains(if cfg!(windows) { "dangle" } else { "? dangle" });

    scene
        .ccmd("chmod")
        .arg("000")
        .arg("some-dir3/some-dir4")
        .succeeds();

    scene
        .ucmd()
        .arg("-laR")
        .arg("some-dir3")
        .fails()
        .code_is(1)
        .stderr_contains("some-dir4")
        .stderr_contains("cannot open directory")
        .stderr_contains("Permission denied")
        .stdout_contains("some-dir4");

    // don't double print on dangling link metadata errors
    scene
        .ucmd()
        .arg("-iRL")
        .arg("some-dir2")
        .fails()
        .stderr_does_not_contain(
            "ls: cannot access 'some-dir2/dangle': No such file or directory\nls: cannot access 'some-dir2/dangle': No such file or directory"
        );

    #[cfg(unix)]
    {
        at.touch("some-dir4/bad-fd.txt");
        let fd1 = at.open("some-dir4/bad-fd.txt").into_raw_fd();
        let fd2 = dup(dbg!(fd1)).unwrap();
        close(fd1).unwrap();

        // on the mac and in certain Linux containers bad fds are typed as dirs,
        // however sometimes bad fds are typed as links and directory entry on links won't fail
        if PathBuf::from(format!("/dev/fd/{fd2}")).is_dir() {
            scene
                .ucmd()
                .arg("-alR")
                .arg(format!("/dev/fd/{fd2}"))
                .fails()
                .stderr_contains(format!(
                    "cannot open directory '/dev/fd/{fd2}': Bad file descriptor"
                ))
                .stdout_does_not_contain(format!("{fd2}:\n"));

            scene
                .ucmd()
                .arg("-RiL")
                .arg(format!("/dev/fd/{fd2}"))
                .fails()
                .stderr_contains(format!("cannot open directory '/dev/fd/{fd2}': Bad file descriptor"))
                // don't double print bad fd errors
                .stderr_does_not_contain(format!("ls: cannot open directory '/dev/fd/{fd2}': Bad file descriptor\nls: cannot open directory '/dev/fd/{fd2}': Bad file descriptor"));
        } else {
            scene
                .ucmd()
                .arg("-alR")
                .arg(format!("/dev/fd/{fd2}"))
                .succeeds();

            scene
                .ucmd()
                .arg("-RiL")
                .arg(format!("/dev/fd/{fd2}"))
                .succeeds();
        }

        scene
            .ucmd()
            .arg("-alL")
            .arg(format!("/dev/fd/{fd2}"))
            .succeeds();

        let _ = close(fd2);
    }
}

#[test]
fn test_ls_only_dirs_formatting() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.mkdir("some-dir2");
    at.mkdir("some-dir3");

    #[cfg(unix)]
    {
        scene.ucmd().arg("-1").arg("-R").succeeds().stdout_only(
            ".:\nsome-dir1\nsome-dir2\nsome-dir3\n\n./some-dir1:\n\n./some-dir2:\n\n./some-dir3:\n",
        );
    }
    #[cfg(windows)]
    {
        scene.ucmd().arg("-1").arg("-R").succeeds().stdout_only(
            ".:\nsome-dir1\nsome-dir2\nsome-dir3\n\n.\\some-dir1:\n\n.\\some-dir2:\n\n.\\some-dir3:\n",
        );
    }
}

#[test]
fn test_ls_walk_glob() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(".test-1");
    at.mkdir("some-dir");
    at.touch(
        Path::new("some-dir")
            .join("test-2~")
            .as_os_str()
            .to_str()
            .unwrap(),
    );

    #[allow(clippy::trivial_regex)]
    let re_pwd = Regex::new(r"^\.\n").unwrap();

    scene
        .ucmd()
        .arg("-1")
        .arg("--ignore-backups")
        .arg("some-dir")
        .succeeds()
        .stdout_does_not_contain("test-2~")
        .stdout_does_not_contain("..")
        .stdout_does_not_match(&re_pwd);
}

#[test]
#[cfg(unix)]
fn test_ls_a() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(".test-1");
    at.mkdir("some-dir");
    at.touch(
        Path::new("some-dir")
            .join(".test-2")
            .as_os_str()
            .to_str()
            .unwrap(),
    );

    #[allow(clippy::trivial_regex)]
    let re_pwd = Regex::new(r"^\.\n").unwrap();

    // Using the present working directory
    scene
        .ucmd()
        .arg("-1")
        .succeeds()
        .stdout_does_not_contain(".test-1")
        .stdout_does_not_contain("..")
        .stdout_does_not_match(&re_pwd);

    scene
        .ucmd()
        .arg("-a")
        .arg("-1")
        .succeeds()
        .stdout_contains(".test-1")
        .stdout_contains("..")
        .stdout_matches(&re_pwd);

    scene
        .ucmd()
        .arg("-A")
        .arg("-1")
        .succeeds()
        .stdout_contains(".test-1")
        .stdout_does_not_contain("..")
        .stdout_does_not_match(&re_pwd);

    // Using a subdirectory
    scene
        .ucmd()
        .arg("-1")
        .arg("some-dir")
        .succeeds()
        .stdout_does_not_contain(".test-2")
        .stdout_does_not_contain("..")
        .stdout_does_not_match(&re_pwd);

    scene
        .ucmd()
        .arg("-a")
        .arg("-1")
        .arg("some-dir")
        .succeeds()
        .stdout_contains(".test-2")
        .stdout_contains("..")
        .no_stderr()
        .stdout_matches(&re_pwd);

    scene
        .ucmd()
        .arg("-A")
        .arg("-1")
        .arg("some-dir")
        .succeeds()
        .stdout_contains(".test-2")
        .stdout_does_not_contain("..")
        .stdout_does_not_match(&re_pwd);
}

#[test]
fn test_ls_width() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-width-1"));
    at.touch(at.plus_as_string("test-width-2"));
    at.touch(at.plus_as_string("test-width-3"));
    at.touch(at.plus_as_string("test-width-4"));

    for option in [
        "-w 100",
        "-w=100",
        "--width=100",
        "--width 100",
        "--wid=100",
    ] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1  test-width-2  test-width-3  test-width-4\n");
    }

    for option in ["-w 50", "-w=50", "--width=50", "--width 50", "--wid=50"] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1  test-width-3\ntest-width-2  test-width-4\n");
    }

    for option in ["-w 25", "-w=25", "--width=25", "--width 25", "--wid=25"] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1\ntest-width-2\ntest-width-3\ntest-width-4\n");
    }

    for option in ["-w 0", "-w=0", "--width=0", "--width 0", "--wid=0"] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1  test-width-2  test-width-3  test-width-4\n");
    }

    for option in [
        "-w 062",
        "-w=062",
        "--width=062",
        "--width 062",
        "--wid=062",
    ] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1  test-width-3\ntest-width-2  test-width-4\n");
    }

    for option in [
        "-w 100000000000000",
        "-w=100000000000000",
        "--width=100000000000000",
        "--width 100000000000000",
        "-w 07777777777777777777",
        "-w=07777777777777777777",
        "--width=07777777777777777777",
        "--width 07777777777777777777",
    ] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .succeeds()
            .stdout_only("test-width-1  test-width-2  test-width-3  test-width-4\n");
    }
    scene
        .ucmd()
        .arg("-w=bad")
        .arg("-C")
        .fails()
        .stderr_contains("invalid line width");

    for option in ["-w 1a", "-w=1a", "--width=1a", "--width 1a", "--wid 1a"] {
        scene
            .ucmd()
            .args(&option.split(' ').collect::<Vec<_>>())
            .arg("-C")
            .fails()
            .stderr_only("ls: invalid line width: '1a'\n");
    }
}

#[test]
fn test_ls_columns() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-columns-1"));
    at.touch(at.plus_as_string("test-columns-2"));
    at.touch(at.plus_as_string("test-columns-3"));
    at.touch(at.plus_as_string("test-columns-4"));

    // Columns is the default
    let result = scene.ucmd().succeeds();

    result.stdout_only("test-columns-1\ntest-columns-2\ntest-columns-3\ntest-columns-4\n");

    for option in COLUMN_ARGS {
        let result = scene.ucmd().arg(option).succeeds();
        result.stdout_only("test-columns-1  test-columns-2  test-columns-3  test-columns-4\n");
    }

    for option in COLUMN_ARGS {
        scene
            .ucmd()
            .arg("-w=40")
            .arg(option)
            .succeeds()
            .stdout_only("test-columns-1  test-columns-3\ntest-columns-2  test-columns-4\n");
    }

    // On windows we are always able to get the terminal size, so we can't simulate falling back to the
    // environment variable.
    #[cfg(not(windows))]
    {
        for option in COLUMN_ARGS {
            scene
                .ucmd()
                .env("COLUMNS", "40")
                .arg(option)
                .succeeds()
                .stdout_only("test-columns-1  test-columns-3\ntest-columns-2  test-columns-4\n");
        }

        scene
            .ucmd()
            .env("COLUMNS", "garbage")
            .arg("-C")
            .succeeds()
            .stdout_is("test-columns-1  test-columns-2  test-columns-3  test-columns-4\n")
            .stderr_is("ls: ignoring invalid width in environment variable COLUMNS: 'garbage'\n");
    }
    scene
        .ucmd()
        .arg("-Cw0")
        .succeeds()
        .stdout_only("test-columns-1  test-columns-2  test-columns-3  test-columns-4\n");
    scene
        .ucmd()
        .arg("-mw0")
        .succeeds()
        .stdout_only("test-columns-1, test-columns-2, test-columns-3, test-columns-4\n");
}

#[test]
fn test_ls_across() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-across-1"));
    at.touch(at.plus_as_string("test-across-2"));
    at.touch(at.plus_as_string("test-across-3"));
    at.touch(at.plus_as_string("test-across-4"));

    for option in ACROSS_ARGS {
        let result = scene.ucmd().arg(option).succeeds();
        // Because the test terminal has width 0, this is the same output as
        // the columns option.
        result.stdout_only("test-across-1  test-across-2  test-across-3  test-across-4\n");
    }

    for option in ACROSS_ARGS {
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
    at.touch(at.plus_as_string("test-commas-1"));
    at.touch(at.plus_as_string("test-commas-2"));
    at.touch(at.plus_as_string("test-commas-3"));
    at.touch(at.plus_as_string("test-commas-4"));

    for option in COMMA_ARGS {
        let result = scene.ucmd().arg(option).succeeds();
        result.stdout_only("test-commas-1, test-commas-2, test-commas-3, test-commas-4\n");
    }

    for option in COMMA_ARGS {
        scene
            .ucmd()
            .arg("-w=30")
            .arg(option)
            .succeeds()
            .stdout_only("test-commas-1, test-commas-2,\ntest-commas-3, test-commas-4\n");
    }
    for option in COMMA_ARGS {
        scene
            .ucmd()
            .arg("-w=45")
            .arg(option)
            .succeeds()
            .stdout_only("test-commas-1, test-commas-2, test-commas-3,\ntest-commas-4\n");
    }
}

#[test]
fn test_ls_zero() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("0-test-zero");
    at.touch(at.plus_as_string("2-test-zero"));
    at.touch(at.plus_as_string("3-test-zero"));

    let ignored_opts = [
        "--quoting-style=c",
        "--color=always",
        "-m",
        "--hide-control-chars",
    ];

    scene
        .ucmd()
        .arg("--zero")
        .succeeds()
        .stdout_only("0-test-zero\x002-test-zero\x003-test-zero\x00");

    for opt in ignored_opts {
        scene
            .ucmd()
            .args(&[opt, "--zero"])
            .succeeds()
            .stdout_only("0-test-zero\x002-test-zero\x003-test-zero\x00");
    }

    scene
        .ucmd()
        .args(&["--zero", "--quoting-style=c"])
        .succeeds()
        .stdout_only("\"0-test-zero\"\x00\"2-test-zero\"\x00\"3-test-zero\"\x00");

    let result = scene.ucmd().args(&["--zero", "--color=always"]).succeeds();
    assert_eq!(
        result.stdout_str(),
        "\u{1b}[0m\u{1b}[01;34m0-test-zero\x1b[0m\x002-test-zero\x003-test-zero\x00"
    );

    scene
        .ucmd()
        .args(&["--zero", "-m"])
        .succeeds()
        .stdout_only("0-test-zero, 2-test-zero, 3-test-zero\x00");

    scene
        .ucmd()
        .args(&["--zero", "--hide-control-chars"])
        .succeeds()
        .stdout_only("0-test-zero\x002-test-zero\x003-test-zero\x00");

    scene
        .ucmd()
        .args(&["--zero", "--quoting-style=c", "--zero"])
        .succeeds()
        .stdout_only("0-test-zero\x002-test-zero\x003-test-zero\x00");

    #[cfg(unix)]
    {
        at.touch(at.plus_as_string("1\ntest-zero"));

        let ignored_opts = [
            "--quoting-style=c",
            "--color=always",
            "--color=alway",
            "--color=al",
            "-m",
            "--hide-control-chars",
        ];

        scene
            .ucmd()
            .arg("--zero")
            .succeeds()
            .stdout_only("0-test-zero\x001\ntest-zero\x002-test-zero\x003-test-zero\x00");

        for opt in ignored_opts {
            scene
                .ucmd()
                .args(&[opt, "--zero"])
                .succeeds()
                .stdout_only("0-test-zero\x001\ntest-zero\x002-test-zero\x003-test-zero\x00");
        }

        scene
            .ucmd()
            .args(&["--zero", "--quoting-style=c"])
            .succeeds()
            .stdout_only(
                "\"0-test-zero\"\x00\"1\\ntest-zero\"\x00\"2-test-zero\"\x00\"3-test-zero\"\x00",
            );

        let result = scene.ucmd().args(&["--zero", "--color=always"]).succeeds();
        assert_eq!(result.stdout_str(),
                "\u{1b}[0m\u{1b}[01;34m0-test-zero\x1b[0m\x001\ntest-zero\x002-test-zero\x003-test-zero\x00",
            );

        scene
            .ucmd()
            .args(&["--zero", "-m"])
            .succeeds()
            .stdout_only("0-test-zero, 1\ntest-zero, 2-test-zero, 3-test-zero\x00");

        scene
            .ucmd()
            .args(&["--zero", "--hide-control-chars"])
            .succeeds()
            .stdout_only("0-test-zero\x001?test-zero\x002-test-zero\x003-test-zero\x00");
    }

    scene
        .ucmd()
        .args(&["-l", "--zero"])
        .succeeds()
        .stdout_contains("total ");
}

#[test]
fn test_ls_commas_trailing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-commas-trailing-2"));

    at.touch(at.plus_as_string("test-commas-trailing-1"));
    at.append(
        "test-commas-trailing-1",
        &(0..2000)
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    );

    scene
        .ucmd()
        .arg("-sm")
        .arg("./test-commas-trailing-1")
        .arg("./test-commas-trailing-2")
        .succeeds()
        .stdout_matches(&Regex::new(r"\S\n$").unwrap());
}

#[test]
fn test_ls_long() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-long"));

    for arg in LONG_ARGS {
        let result = scene.ucmd().arg(arg).arg("test-long").succeeds();
        #[cfg(not(windows))]
        result.stdout_matches(&Regex::new(r"[-bcCdDlMnpPsStTx?]([r-][w-][xt-]){3}.*").unwrap());

        #[cfg(windows)]
        result.stdout_matches(&Regex::new(r"[-dl](r[w-]x){3}.*").unwrap());
    }
}

#[cfg(not(windows))]
#[test]
fn test_ls_long_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir(at.plus_as_string("test-long-dir"));
    at.touch(at.plus_as_string("test-long-dir/test-long-file"));
    at.mkdir(at.plus_as_string("test-long-dir/test-long-dir"));

    for arg in LONG_ARGS {
        // Assuming sane username do not have spaces within them.
        // A line of the output should be:
        // One of the characters -bcCdDlMnpPsStTx?
        // rwx, with - for missing permissions, thrice.
        // Zero or one "." for indicating a file with security context
        // A number, preceded by column whitespace, and followed by a single space.
        // A username, currently [^ ], followed by column whitespace, twice (or thrice for Hurd).
        // A number, followed by a single space.
        // A month, followed by a single space.
        // A day, preceded by column whitespace, and followed by a single space.
        // Either a year or a time, currently [0-9:]+, preceded by column whitespace,
        // and followed by a single space.
        // Whatever comes after is irrelevant to this specific test.
        scene.ucmd().arg(arg).arg("test-long-dir").succeeds().stdout_matches(&Regex::new(
            r"\n[-bcCdDlMnpPsStTx?]([r-][w-][xt-]){3}\.? +\d+ [^ ]+ +[^ ]+( +[^ ]+)? +\d+ [A-Z][a-z]{2} {0,2}\d{0,2} {0,2}[0-9:]+ "
        ).unwrap());
    }

    // This checks for the line with the .. entry. The uname and group should be digits.
    scene.ucmd().arg("-lan").arg("test-long-dir").succeeds().stdout_matches(&Regex::new(
        r"\nd([r-][w-][xt-]){3}\.? +\d+ \d+ +\d+( +\d+)? +\d+ [A-Z][a-z]{2} {0,2}\d{0,2} {0,2}[0-9:]+ \.\."
    ).unwrap());
}

#[test]
fn test_ls_long_padding_of_size_column_with_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir");
    at.touch("dir/a");
    at.touch("dir/b");

    ucmd.arg("-l")
        .arg("dir")
        .succeeds()
        .stdout_contains(" 0 ")
        .stdout_does_not_contain("  0 ");
}

/// This test tests `ls -laR --color`.
/// This test is mainly about coloring, but, the recursion, symlink `->` processing,
/// and `.` and `..` being present in `-a` all need to work for the test to pass.
/// This test does not really test anything provided by `-l` but the file names and symlinks.
#[cfg(all(feature = "ln", feature = "mkdir", feature = "touch"))]
#[test]
#[cfg(all(feature = "ln", feature = "mkdir", feature = "touch"))]
fn test_ls_long_symlink_color() {
    // If you break this test after breaking mkdir, touch, or ln, do not be alarmed!
    // This test is made for ls, but it attempts to run those utils in the process.

    // Having Some([2, 0]) in a color basically means that "it has the same color as whatever
    // is in the 2nd expected output, the 0th color", where the 0th color is the name color, and
    // the 1st color is the target color, in a fixed-size array of size 2.
    // Basically these are references to be used for indexing the `colors` vector defined below.
    type ColorReference = Option<[usize; 2]>;

    // The string between \x1b[ and m
    type Color = String;

    // The string between the color start and the color end is the file name itself.
    type Name = String;

    let scene = TestScenario::new(util_name!());

    // .
    // ├── dir1
    // │   ├── file1
    // │   ├── dir2
    // │   │   └── dir3
    // │   ├── ln-dir-invalid -> dir1/dir2
    // │   ├── ln-up2 -> ../..
    // │   └── ln-root -> /
    // ├── ln-file1 -> dir1/file1
    // ├── ln-file-invalid -> dir1/invalid-target
    // └── ln-dir3 -> ./dir1/dir2/dir3
    prepare_folder_structure(&scene);

    // We memoize the colors so we can refer to them later.
    // Each entry will be the colors of the link name and link target of a specific output.
    let mut colors: Vec<[Color; 2]> = vec![];

    // The contents of each tuple are the expected colors and names for the link and target.
    // We will loop over the ls output and compare to those.
    // None values mean that we do not know what color to expect yet, as LS_COLOR might
    // be set differently, and as different implementations of ls may use different codes,
    // for example, our ls uses `[1;36m` while the GNU ls uses `[01;36m`.
    //
    // These have been sorting according to default ls sort, and this affects the order of
    // discovery of colors, so be very careful when changing directory/file names being created.
    let expected_output: [(ColorReference, &str, ColorReference, &str); 6] = [
        // We don't know what colors are what the first time we meet a link.
        (None, "ln-dir3", None, "./dir1/dir2/dir3"),
        // We have acquired [0, 0], which should be the link color,
        // and [0, 1], which should be the dir color, and we can compare to them from now on.
        (None, "ln-file-invalid", Some([1, 1]), "dir1/invalid-target"),
        // We acquired [1, 1], the non-existent color.
        (Some([0, 0]), "ln-file1", None, "dir1/file1"),
        (Some([1, 1]), "ln-dir-invalid", Some([1, 1]), "dir1/dir2"),
        (Some([0, 0]), "ln-root", Some([0, 1]), "/"),
        (Some([0, 0]), "ln-up2", None, "../.."),
    ];

    // We are only interested in lines or the ls output that are symlinks. These start with "lrwx".
    let result = scene.ucmd().arg("-laR").arg("--color").arg(".").succeeds();
    let mut result_lines = result
        .stdout_str()
        .lines()
        .filter(|line| line.starts_with("lrwx"))
        .enumerate();

    // For each enumerated line, we assert that the output of ls matches the expected output.
    //
    // The unwraps within get_index_name_target will panic if a line starting lrwx does
    // not have `colored_name -> target` within it.
    while let Some((i, name, target)) = get_index_name_target(&mut result_lines) {
        // The unwraps within capture_colored_string will panic if the name/target's color
        // format is invalid.
        dbg!(&name);
        dbg!(&target);
        let (matched_name_color, matched_name) = capture_colored_string(&name);
        let (matched_target_color, matched_target) = capture_colored_string(&target);

        colors.push([matched_name_color, matched_target_color]);

        // We borrow them again after having moved them. This unwrap will never panic.
        let [matched_name_color, matched_target_color] = colors.last().unwrap();

        // We look up the Colors that are expected in `colors` using the ColorReferences
        // stored in `expected_output`.
        let expected_name_color = expected_output[i]
            .0
            .map(|color_reference| colors[color_reference[0]][color_reference[1]].as_str());
        let expected_target_color = expected_output[i]
            .2
            .map(|color_reference| colors[color_reference[0]][color_reference[1]].as_str());

        // This is the important part. The asserts inside assert_names_and_colors_are_equal
        // will panic if the colors or names do not match the expected colors or names.
        // Keep in mind an expected color `Option<&str>` of None can mean either that we
        // don't expect any color here, as in `expected_output[2], or don't know what specific
        // color to expect yet, as in expected_output[0:1].
        dbg!(&colors);
        assert_names_and_colors_are_equal(
            matched_name_color,
            expected_name_color,
            &matched_name,
            expected_output[i].1,
            matched_target_color,
            expected_target_color,
            &matched_target,
            expected_output[i].3,
        );
    }

    // End of test, only definitions of the helper functions used above follows...

    fn get_index_name_target<'a, I>(lines: &mut I) -> Option<(usize, Name, Name)>
    where
        I: Iterator<Item = (usize, &'a str)>,
    {
        match lines.next() {
            Some((c, s)) => {
                // `name` is whatever comes between \x1b (inclusive) and the arrow.
                let name = String::from("\x1b")
                    + s.split(" -> ")
                        .next()
                        .unwrap()
                        .split(" \x1b")
                        .last()
                        .unwrap();
                // `target` is whatever comes after the arrow.
                let target = s.split(" -> ").last().unwrap().to_string();
                Some((c, name, target))
            }
            None => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_names_and_colors_are_equal(
        name_color: &str,
        expected_name_color: Option<&str>,
        name: &str,
        expected_name: &str,
        target_color: &str,
        expected_target_color: Option<&str>,
        target: &str,
        expected_target: &str,
    ) {
        // Names are always compared.
        assert_eq!(&name, &expected_name);
        assert_eq!(&target, &expected_target);

        // Colors are only compared when we have inferred what color we are looking for.
        if expected_name_color.is_some() {
            assert_eq!(&name_color, &expected_name_color.unwrap());
        }
        if expected_target_color.is_some() {
            assert_eq!(&target_color, &expected_target_color.unwrap());
        }
    }

    fn capture_colored_string(input: &str) -> (Color, Name) {
        // Input can be:
        // \u{1b}[0m\u{1b}[01;36mln-dir3\u{1b}[0m
        // \u{1b}[0m\u{1b}[01;34m./dir1/dir2/dir3\u{1b}[0m
        // \u{1b}[0m\u{1b}[01;36mln-file-invalid\u{1b}[0m
        // \u{1b}[01;36mdir1/invalid-target\u{1b}[0m
        let colored_name = Regex::new(r"(?:\x1b\[0m\x1b)?\[([0-9;]+)m(.+)\x1b\[0m").unwrap();
        match colored_name.captures(input) {
            Some(captures) => {
                dbg!(captures.get(1).unwrap().as_str().to_string());
                dbg!(captures.get(2).unwrap().as_str().to_string());
                return (
                    captures.get(1).unwrap().as_str().to_string(),
                    captures.get(2).unwrap().as_str().to_string(),
                );
            }
            None => (String::new(), input.to_string()),
        }
    }

    fn prepare_folder_structure(scene: &TestScenario) {
        // There is no way to change directory in the CI, so this is the best we can do.
        // Also, keep in mind that windows might require privilege to symlink directories.
        //
        // We use scene.ccmd instead of scene.fixtures because we care about relative symlinks.
        // So we're going to try out the built mkdir, touch, and ln here, and we expect them to succeed.
        scene.ccmd("mkdir").arg("dir1").succeeds();
        scene.ccmd("mkdir").arg("dir1/dir2").succeeds();
        scene.ccmd("mkdir").arg("dir1/dir2/dir3").succeeds();
        scene.ccmd("touch").arg("dir1/file1").succeeds();

        scene
            .ccmd("ln")
            .arg("-s")
            .arg("dir1/dir2")
            .arg("dir1/ln-dir-invalid")
            .succeeds();
        scene
            .ccmd("ln")
            .arg("-s")
            .arg("./dir1/dir2/dir3")
            .arg("ln-dir3")
            .succeeds();
        scene
            .ccmd("ln")
            .arg("-s")
            .arg("../..")
            .arg("dir1/ln-up2")
            .succeeds();
        scene
            .ccmd("ln")
            .arg("-s")
            .arg("/")
            .arg("dir1/ln-root")
            .succeeds();
        scene
            .ccmd("ln")
            .arg("-s")
            .arg("dir1/file1")
            .arg("ln-file1")
            .succeeds();
        scene
            .ccmd("ln")
            .arg("-s")
            .arg("dir1/invalid-target")
            .arg("ln-file-invalid")
            .succeeds();
    }
}

#[test]
fn test_ls_long_total_size() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-long"));
    at.append("test-long", "1");
    at.touch(at.plus_as_string("test-long2"));
    at.append("test-long2", "2");

    let expected_prints: HashMap<_, _> = if cfg!(unix) {
        [
            ("long_vanilla", "total 8"),
            ("long_human_readable", "total 8.0K"),
            ("long_si", "total 8.2k"),
        ]
        .iter()
        .copied()
        .collect()
    } else {
        [
            ("long_vanilla", "total 2"),
            ("long_human_readable", "total 2"),
            ("long_si", "total 2"),
        ]
        .iter()
        .copied()
        .collect()
    };

    for arg in LONG_ARGS {
        let result = scene.ucmd().arg(arg).succeeds();
        result.stdout_contains(expected_prints["long_vanilla"]);

        for arg2 in ["-h", "--human-readable", "--si"] {
            let result = scene.ucmd().arg(arg).arg(arg2).succeeds();
            result.stdout_contains(if arg2 == "--si" {
                expected_prints["long_si"]
            } else {
                expected_prints["long_human_readable"]
            });
        }
    }
}

#[test]
fn test_ls_long_formats() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-long-formats"));

    // Zero or one "." for indicating a file with security context

    // Regex for three names, so all of author, group and owner
    let re_three = Regex::new(r"[xrw-]{9}\.? \d ([-0-9_a-z.A-Z]+ ){3}0").unwrap();

    #[cfg(unix)]
    let re_three_num = Regex::new(r"[xrw-]{9}\.? \d (\d+ ){3}0").unwrap();

    // Regex for two names, either:
    // - group and owner
    // - author and owner
    // - author and group
    let re_two = Regex::new(r"[xrw-]{9}\.? \d ([-0-9_a-z.A-Z]+ ){2}0").unwrap();

    #[cfg(unix)]
    let re_two_num = Regex::new(r"[xrw-]{9}\.? \d (\d+ ){2}0").unwrap();

    // Regex for one name: author, group or owner
    let re_one = Regex::new(r"[xrw-]{9}\.? \d [-0-9_a-z.A-Z]+ 0").unwrap();

    #[cfg(unix)]
    let re_one_num = Regex::new(r"[xrw-]{9}\.? \d \d+ 0").unwrap();

    // Regex for no names
    let re_zero = Regex::new(r"[xrw-]{9}\.? \d 0").unwrap();

    scene
        .ucmd()
        .arg("-l")
        .arg("--author")
        .arg("test-long-formats")
        .succeeds()
        .stdout_matches(&re_three);

    scene
        .ucmd()
        .arg("-l1")
        .arg("--author")
        .arg("test-long-formats")
        .succeeds()
        .stdout_matches(&re_three);

    #[cfg(unix)]
    {
        scene
            .ucmd()
            .arg("-n")
            .arg("--author")
            .arg("test-long-formats")
            .succeeds()
            .stdout_matches(&re_three_num);
    }

    for arg in [
        "-l",                     // only group and owner
        "-g --author",            // only author and group
        "-o --author",            // only author and owner
        "-lG --author",           // only author and owner
        "-l --no-group --author", // only author and owner
    ] {
        scene
            .ucmd()
            .args(&arg.split(' ').collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds()
            .stdout_matches(&re_two);

        #[cfg(unix)]
        {
            scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(' ').collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds()
                .stdout_matches(&re_two_num);
        }
    }

    for arg in [
        "-g",            // only group
        "-gl",           // only group
        "-o",            // only owner
        "-ol",           // only owner
        "-oG",           // only owner
        "-lG",           // only owner
        "-l --no-group", // only owner
        "-gG --author",  // only author
    ] {
        scene
            .ucmd()
            .args(&arg.split(' ').collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds()
            .stdout_matches(&re_one);

        #[cfg(unix)]
        {
            scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(' ').collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds()
                .stdout_matches(&re_one_num);
        }
    }

    for arg in [
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
        scene
            .ucmd()
            .args(&arg.split(' ').collect::<Vec<_>>())
            .arg("test-long-formats")
            .succeeds()
            .stdout_matches(&re_zero);

        #[cfg(unix)]
        {
            scene
                .ucmd()
                .arg("-n")
                .args(&arg.split(' ').collect::<Vec<_>>())
                .arg("test-long-formats")
                .succeeds()
                .stdout_matches(&re_zero);
        }
    }
}

#[test]
fn test_ls_oneline() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("test-oneline-1"));
    at.touch(at.plus_as_string("test-oneline-2"));

    // Bit of a weird situation: in the tests oneline and columns have the same output,
    // except on Windows.
    for option in ["-1", "--format=single-column"] {
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

    at.touch(at.plus_as_string("test-long"));
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
        .arg("-l")
        .arg("--color=neve") // spell-checker:disable-line
        .arg("test-long")
        .arg("test-long.link")
        .succeeds();
    assert!(re.is_match(result.stdout_str().trim()));

    let result = scene
        .ucmd()
        .arg("-l")
        .arg("--color=n")
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
fn test_ls_group_directories_first() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let mut filenames = ["file1", "file2", "anotherFile", "abc", "xxx", "zzz"];
    for filename in filenames {
        at.touch(filename);
    }
    filenames.sort_unstable();

    let dirnames = ["aaa", "bbb", "ccc", "yyy"];
    for dirname in dirnames {
        at.mkdir(dirname);
    }

    let dots = [".", ".."];

    let result = scene
        .ucmd()
        .arg("-1a")
        .arg("--group-directories-first")
        .run();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        dots.into_iter()
            .chain(dirnames.into_iter())
            .chain(filenames.into_iter())
            .chain([""].into_iter())
            .collect::<Vec<_>>(),
    );

    let result = scene
        .ucmd()
        .arg("-1ar")
        .arg("--group-directories-first")
        .run();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        (dirnames.into_iter().rev())
            .chain(dots.into_iter().rev())
            .chain(filenames.into_iter().rev())
            .chain([""].into_iter())
            .collect::<Vec<_>>(),
    );

    let result = scene
        .ucmd()
        .arg("-1aU")
        .arg("--group-directories-first")
        .run();
    let result2 = scene.ucmd().arg("-1aU").run();
    assert_eq!(result.stdout_str(), result2.stdout_str());
}
#[test]
fn test_ls_sort_none() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-3");
    at.touch("test-1");
    at.touch("test-2");

    // Order is not specified so we just check that it doesn't
    // give any errors.
    scene.ucmd().arg("--sort=none").succeeds();
    scene.ucmd().arg("--sort=non").succeeds();
    scene.ucmd().arg("--sort=no").succeeds();
    // scene.ucmd().arg("--sort=n").succeeds();
    // We refuse to accept "--sort=n", since this is too confusable with "--sort=name", which is our own extension.
    scene.ucmd().arg("-U").succeeds();
}

#[test]
fn test_ls_sort_name() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-3");
    at.touch("test-1");
    at.touch("test-2");

    scene
        .ucmd()
        .arg("--sort=name")
        .succeeds()
        .stdout_is("test-1\ntest-2\ntest-3\n");
    scene
        .ucmd()
        .arg("--sort=nam")
        .succeeds()
        .stdout_is("test-1\ntest-2\ntest-3\n");
    scene
        .ucmd()
        .arg("--sort=na")
        .succeeds()
        .stdout_is("test-1\ntest-2\ntest-3\n");

    let scene_dot = TestScenario::new(util_name!());
    let at = &scene_dot.fixtures;
    at.touch(".a");
    at.touch("a");
    at.touch(".b");
    at.touch("b");

    scene_dot
        .ucmd()
        .arg("--sort=name")
        .arg("-A")
        .succeeds()
        .stdout_is(".a\n.b\na\nb\n");
}

#[test]
fn test_ls_sort_width() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("aaaaa");
    at.touch("bbb");
    at.touch("cccc");
    at.touch("eee");
    at.touch("d");
    at.touch("fffff");
    at.touch("abc");
    at.touch("zz");
    at.touch("bcdef");

    scene
        .ucmd()
        .arg("--sort=width")
        .succeeds()
        .stdout_is("d\nzz\nabc\nbbb\neee\ncccc\naaaaa\nbcdef\nfffff\n");
    scene
        .ucmd()
        .arg("--sort=widt") // spell-checker:disable-line
        .succeeds()
        .stdout_is("d\nzz\nabc\nbbb\neee\ncccc\naaaaa\nbcdef\nfffff\n");
    scene
        .ucmd()
        .arg("--sort=w")
        .succeeds()
        .stdout_is("d\nzz\nabc\nbbb\neee\ncccc\naaaaa\nbcdef\nfffff\n");
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
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("-S").arg("-r").succeeds();
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");

    let result = scene.ucmd().arg("--sort=size").succeeds();
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("--sort=siz").succeeds();
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("--sort=s").succeeds();
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("--sort=size").arg("-r").succeeds();
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");
}

#[test]
fn test_ls_long_ctime() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("test-long-ctime-1");

    for arg in ["-c", "--time=ctime", "--time=status"] {
        let result = scene.ucmd().arg("-l").arg(arg).succeeds();

        // Should show the time on Unix, but question marks on windows.
        #[cfg(unix)]
        result.stdout_contains(":");
        #[cfg(not(unix))]
        result.stdout_contains("???");
    }
}

#[test]
#[ignore]
fn test_ls_order_birthtime() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    /*
        Here we make 2 files with a timeout in between.
        After creating the first file try to sync it.
        This ensures the file gets created immediately instead of being saved
        inside the OS's IO operation buffer.
        Without this, both files might accidentally be created at the same time.
    */
    at.make_file("test-birthtime-1").sync_all().unwrap();
    at.make_file("test-birthtime-2").sync_all().unwrap();
    at.open("test-birthtime-1");

    let result = scene.ucmd().arg("--time=birth").arg("-t").run();

    #[cfg(not(windows))]
    assert_eq!(result.stdout_str(), "test-birthtime-2\ntest-birthtime-1\n");
    #[cfg(windows)]
    assert_eq!(result.stdout_str(), "test-birthtime-2  test-birthtime-1\n");
}

#[test]
fn test_ls_styles() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("test");

    let re_full = Regex::new(
        r"[a-z-]* \d* [\w.]* [\w.]* \d* \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d* (\+|\-)\d{4} test\n",
    )
    .unwrap();
    let re_long =
        Regex::new(r"[a-z-]* \d* [\w.]* [\w.]* \d* \d{4}-\d{2}-\d{2} \d{2}:\d{2} test\n").unwrap();
    let re_iso =
        Regex::new(r"[a-z-]* \d* [\w.]* [\w.]* \d* \d{2}-\d{2} \d{2}:\d{2} test\n").unwrap();
    let re_locale =
        Regex::new(r"[a-z-]* \d* [\w.]* [\w.]* \d* [A-Z][a-z]{2} ( |\d)\d \d{2}:\d{2} test\n")
            .unwrap();
    let re_custom_format =
        Regex::new(r"[a-z-]* \d* [\w.]* [\w.]* \d* \d{4}__\d{2} test\n").unwrap();

    //full-iso
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=full-iso")
        .succeeds()
        .stdout_matches(&re_full);
    //long-iso
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=long-iso")
        .succeeds()
        .stdout_matches(&re_long);
    //iso
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=iso")
        .succeeds()
        .stdout_matches(&re_iso);
    //locale
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=locale")
        .succeeds()
        .stdout_matches(&re_locale);

    //+FORMAT
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=+%Y__%M")
        .succeeds()
        .stdout_matches(&re_custom_format);

    // Also fails due to not having full clap support for time_styles
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=invalid")
        .fails()
        .code_is(2);

    //Overwrite options tests
    scene
        .ucmd()
        .arg("-l")
        .arg("--time-style=long-iso")
        .arg("--time-style=iso")
        .succeeds()
        .stdout_matches(&re_iso);
    scene
        .ucmd()
        .arg("--time-style=iso")
        .arg("--full-time")
        .succeeds()
        .stdout_matches(&re_full);
    scene
        .ucmd()
        .arg("--full-time")
        .arg("--time-style=iso")
        .succeeds()
        .stdout_matches(&re_iso);

    scene
        .ucmd()
        .arg("--full-time")
        .arg("--time-style=iso")
        .arg("--full-time")
        .succeeds()
        .stdout_matches(&re_full);

    scene
        .ucmd()
        .arg("--full-time")
        .arg("-x")
        .arg("-l")
        .succeeds()
        .stdout_matches(&re_full);

    at.touch("test2");
    scene
        .ucmd()
        .arg("--full-time")
        .arg("-x")
        .succeeds()
        .stdout_is("test  test2\n");
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
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("--sort=time").succeeds();
    result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");

    let result = scene.ucmd().arg("-tr").succeeds();
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");

    let result = scene.ucmd().arg("--sort=time").arg("-r").succeeds();
    result.stdout_only("test-1\ntest-2\ntest-3\ntest-4\n");

    // 3 was accessed last in the read
    // So the order should be 2 3 4 1
    for arg in [
        "-u",
        "--time=atime",
        "--time=atim", // spell-checker:disable-line
        "--time=a",
        "--time=access",
        "--time=use",
    ] {
        let result = scene.ucmd().arg("-t").arg(arg).succeeds();
        at.open("test-3").metadata().unwrap().accessed().unwrap();
        at.open("test-4").metadata().unwrap().accessed().unwrap();

        // It seems to be dependent on the platform whether the access time is actually set
        #[cfg(unix)]
        {
            let expected = unwrap_or_return!(expected_result(&scene, &["-t", arg]));
            at.open("test-3").metadata().unwrap().accessed().unwrap();
            at.open("test-4").metadata().unwrap().accessed().unwrap();

            result.stdout_only(expected.stdout_str());
        }
        #[cfg(windows)]
        result.stdout_only("test-4\ntest-3\ntest-2\ntest-1\n");
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
    at.touch(at.plus_as_string("a/a"));
    at.touch(at.plus_as_string("a/b/b"));

    scene.ucmd().arg("a").succeeds();
    scene.ucmd().arg("a/a").succeeds();
    scene.ucmd().arg("a").arg("z").succeeds();

    // Doesn't exist
    scene
        .ucmd()
        .arg("doesntexist")
        .fails()
        .stderr_contains("'doesntexist': No such file or directory");

    // One exists, the other doesn't
    scene
        .ucmd()
        .arg("a")
        .arg("doesntexist")
        .fails()
        .stderr_contains("'doesntexist': No such file or directory")
        .stdout_contains("a:");
}

#[test]
fn test_ls_recursive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    at.touch(at.plus_as_string("a/a"));
    at.touch(at.plus_as_string("a/b/b"));

    scene.ucmd().arg("a").succeeds();
    scene.ucmd().arg("a/a").succeeds();
    scene
        .ucmd()
        .arg("z")
        .arg("-R")
        .succeeds()
        .stdout_contains("z:");
    let result = scene
        .ucmd()
        .arg("--color=never")
        .arg("-R")
        .arg("a")
        .arg("z")
        .succeeds();

    #[cfg(not(windows))]
    result.stdout_contains("a/b:\nb");
    #[cfg(windows)]
    result.stdout_contains("a\\b:\nb");
}

#[test]
fn test_ls_recursive_1() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("x");
    at.mkdir("y");
    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.mkdir("a/1");
    at.mkdir("a/2");
    at.mkdir("a/3");
    at.touch("f");
    at.touch("a/1/I");
    at.touch("a/1/II");
    #[cfg(unix)]
    let out = "a:\n1\n2\n3\n\na/1:\nI\nII\n\na/2:\n\na/3:\n\nb:\n\nc:\n";
    #[cfg(windows)]
    let out = "a:\n1\n2\n3\n\na\\1:\nI\nII\n\na\\2:\n\na\\3:\n\nb:\n\nc:\n";
    scene
        .ucmd()
        .arg("-R1")
        .arg("a")
        .arg("b")
        .arg("c")
        .succeeds()
        .stdout_is(out);
}

#[test]
fn test_ls_color() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    let nested_dir = Path::new("a")
        .join("nested_dir")
        .to_string_lossy()
        .to_string();
    at.mkdir(nested_dir);
    at.mkdir("z");
    let nested_file = Path::new("a")
        .join("nested_file")
        .to_string_lossy()
        .to_string();
    at.touch(nested_file);
    at.touch("test-color");

    let a_with_colors = "\x1b[0m\x1b[01;34ma\x1b[0m";
    let z_with_colors = "\x1b[01;34mz\x1b[0m\n";
    let nested_dir_with_colors = "\x1b[0m\x1b[01;34mnested_dir\x1b[0m\x0anested_file"; // spell-checker:disable-line

    // Color is disabled by default
    let result = scene.ucmd().succeeds();
    assert!(!result.stdout_str().contains(a_with_colors));
    assert!(!result.stdout_str().contains(z_with_colors));

    // Color should be enabled
    for param in ["--color", "--col", "--color=always", "--col=always"] {
        let result = scene.ucmd().arg(param).succeeds();
        assert!(result.stdout_str().contains(a_with_colors));
        assert!(result.stdout_str().contains(z_with_colors));
    }

    // Color should be disabled
    let result = scene.ucmd().arg("--color=never").succeeds();
    assert!(!result.stdout_str().contains(a_with_colors));
    assert!(!result.stdout_str().contains(z_with_colors));

    // Nested dir should be shown and colored
    let result = scene.ucmd().arg("--color").arg("a").succeeds();
    assert!(result.stdout_str().contains(nested_dir_with_colors));

    // No output
    scene
        .ucmd()
        .arg("--color=never")
        .arg("z")
        .succeeds()
        .stdout_only("");

    // The colors must not mess up the grid layout
    at.touch("b");
    let result = scene
        .ucmd()
        .arg("--color")
        .arg("-w=15")
        .arg("-C")
        .succeeds();
    let expected = format!("{}  test-color\x0ab  {}", a_with_colors, z_with_colors);
    assert_eq!(
        result.stdout_str().escape_default().to_string(),
        expected.escape_default().to_string()
    );
    assert_eq!(result.stdout_str(), expected);
}

#[cfg(unix)]
#[test]
fn test_ls_inode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "test_inode";
    at.touch(file);

    let re_short = Regex::new(r" *(\d+) test_inode").unwrap();
    let re_long = Regex::new(r" *(\d+) [xrw-]{10}\.? \d .+ test_inode").unwrap();

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

    assert_eq!(inode_short, inode_long);
}

#[test]
#[cfg(not(windows))]
fn test_ls_indicator_style() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Setup: Directory, Symlink, and Pipes.
    at.mkdir("directory");
    assert!(at.dir_exists("directory"));

    at.touch(at.plus_as_string("link-src"));
    at.symlink_file("link-src", "link-dest.link");
    assert!(at.is_symlink("link-dest.link"));

    at.mkfifo("named-pipe.fifo");
    assert!(at.is_fifo("named-pipe.fifo"));

    // Classify, File-Type, and Slash all contain indicators for directories.
    for opt in [
        "--indicator-style=classify",
        "--ind=classify",
        "--indicator-style=clas", // spell-checker:disable-line
        "--indicator-style=c",
        "--indicator-style=file-type",
        "--ind=file-type",
        "--indicator-style=slash",
        "--ind=slash",
        "--classify",
        "--classify=always",
        "--classify=alway", // spell-checker:disable-line
        "--classify=al",
        "--classify=yes",
        "--classify=force",
        "--class",
        "--file-type",
        "--file",
        "-p",
    ] {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene.ucmd().arg(opt).succeeds().stdout_contains("/");
    }

    // Classify, Indicator options should not contain any indicators when value is none.
    for opt in [
        "--indicator-style=none",
        "--indicator-style=n",
        "--ind=none",
        "--classify=none",
        "--classify=never",
        "--classify=non",
        "--classify=no",
        "--classify=n",
    ] {
        // Verify that there are no indicators for any of the file types.
        scene
            .ucmd()
            .arg(opt)
            .succeeds()
            .stdout_does_not_contain("/")
            .stdout_does_not_contain("@")
            .stdout_does_not_contain("|");
    }

    // Classify and File-Type all contain indicators for pipes and links.
    let options = vec!["classify", "file-type"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene
            .ucmd()
            .arg(format!("--indicator-style={opt}"))
            .succeeds()
            .stdout_contains("@")
            .stdout_contains("|");
    }

    // Test sockets. Because the canonical way of making sockets to test is with
    // TempDir, we need a separate test.
    {
        use std::os::unix::net::UnixListener;

        let dir = tempfile::Builder::new()
            .prefix("unix_socket")
            .tempdir()
            .expect("failed to create dir");
        let socket_path = dir.path().join("sock");
        let _listener = UnixListener::bind(socket_path).expect("failed to create socket");

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

    at.touch(at.plus_as_string("link-src"));
    at.symlink_file("link-src", "link-dest.link");
    assert!(at.is_symlink("link-dest.link"));

    // Classify, File-Type, and Slash all contain indicators for directories.
    let options = vec!["classify", "file-type", "slash"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene
            .ucmd()
            .arg(format!("--indicator-style={opt}"))
            .succeeds()
            .stdout_contains("/");
    }

    // Same test as above, but with the alternate flags.
    let options = vec!["--classify", "--file-type", "-p"];
    for opt in options {
        scene.ucmd().arg(opt).succeeds().stdout_contains("/");
    }

    // Classify and File-Type all contain indicators for pipes and links.
    let options = vec!["classify", "file-type"];
    for opt in options {
        // Verify that classify and file-type both contain indicators for symlinks.
        scene
            .ucmd()
            .arg(format!("--indicator-style={opt}"))
            .succeeds()
            .stdout_contains("@");
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
    scene.ucmd().arg("-a").succeeds().stdout_contains(file);
}

#[cfg(windows)]
#[test]
fn test_ls_hidden_link_windows() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file = "visibleWindowsFileNoDot";
    at.touch(file);

    let link = "hiddenWindowsLinkNoDot";
    at.symlink_dir(file, link);
    // hide the link
    scene.cmd("attrib").arg("/l").arg("+h").arg(link).succeeds();

    scene
        .ucmd()
        .succeeds()
        .stdout_contains(file)
        .stdout_does_not_contain(link);

    scene
        .ucmd()
        .arg("-a")
        .succeeds()
        .stdout_contains(file)
        .stdout_contains(link);
}

#[cfg(windows)]
#[test]
fn test_ls_success_on_c_drv_root_windows() {
    let scene = TestScenario::new(util_name!());
    scene.ucmd().arg("C:\\").succeeds();
}

#[test]
fn test_ls_version_sort() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    for filename in [
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

    let result = scene.ucmd().arg("-1").arg("--sort=v").succeeds();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected
    );

    let result = scene.ucmd().arg("-a1v").succeeds();
    expected.insert(expected.len() - 1, "..");
    expected.insert(0, ".");
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected,
    );
}

#[test]
fn test_ls_quoting_style() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("one two");
    at.touch("one");

    // It seems that windows doesn't allow \n in filenames.
    // And it also doesn't like \, of course.
    #[cfg(unix)]
    {
        at.touch("one\ntwo");
        at.touch("one\\two");
        // Default is literal, when stdout is not a TTY.
        // Otherwise, it is shell-escape
        scene
            .ucmd()
            .arg("--hide-control-chars")
            .arg("one\ntwo")
            .succeeds()
            .stdout_only("one?two\n");
        // TODO: TTY-expected output, find a way to check this as well
        // .stdout_only("'one'$'\\n''two'\n");

        for (arg, correct) in [
            ("--quoting-style=literal", "one?two"),
            ("--quoting-style=litera", "one?two"), // spell-checker:disable-line
            ("--quoting-style=li", "one?two"),
            ("-N", "one?two"),
            ("--literal", "one?two"),
            ("--l", "one?two"),
            ("--quoting-style=c", "\"one\\ntwo\""),
            ("--quoting-style=c-", "\"one\\ntwo\""),
            ("--quoting-style=c-maybe", "\"one\\ntwo\""),
            ("-Q", "\"one\\ntwo\""),
            ("--quote-name", "\"one\\ntwo\""),
            ("--quoting-style=escape", "one\\ntwo"),
            ("--quoting-style=escap", "one\\ntwo"), // spell-checker:disable-line
            ("-b", "one\\ntwo"),
            ("--escape", "one\\ntwo"),
            ("--quoting-style=shell-escape", "'one'$'\\n''two'"),
            ("--quoting-style=shell-escape-always", "'one'$'\\n''two'"),
            ("--quoting-style=shell-escape-alway", "'one'$'\\n''two'"),
            ("--quoting-style=shell-escape-a", "'one'$'\\n''two'"),
            ("--quoting-style=shell", "one?two"),
            ("--quoting-style=shell-always", "'one?two'"),
            ("--quoting-style=shell-a", "'one?two'"),
        ] {
            scene
                .ucmd()
                .arg("--hide-control-chars")
                .arg(arg)
                .arg("one\ntwo")
                .succeeds()
                .stdout_only(format!("{correct}\n"));
        }

        for (arg, correct) in [
            ("--quoting-style=literal", "one\ntwo"),
            ("-N", "one\ntwo"),
            ("--literal", "one\ntwo"),
            ("--l", "one\ntwo"),
            ("--quoting-style=shell", "one\ntwo"), // FIXME: GNU ls quotes this case
            ("--quoting-style=shell-always", "'one\ntwo'"),
        ] {
            scene
                .ucmd()
                .arg(arg)
                .arg("--show-control-chars")
                .arg("one\ntwo")
                .succeeds()
                .stdout_only(format!("{correct}\n"));
        }

        for (arg, correct) in [
            ("--quoting-style=literal", "one\\two"),
            ("-N", "one\\two"),
            ("--quoting-style=c", "\"one\\\\two\""),
            ("-Q", "\"one\\\\two\""),
            ("--quote-name", "\"one\\\\two\""),
            ("--quoting-style=escape", "one\\\\two"),
            ("-b", "one\\\\two"),
            ("--quoting-style=shell-escape", "'one\\two'"),
            ("--quoting-style=shell-escape-always", "'one\\two'"),
            ("--quoting-style=shell", "'one\\two'"),
            ("--quoting-style=shell-always", "'one\\two'"),
        ] {
            scene
                .ucmd()
                .arg("--hide-control-chars")
                .arg(arg)
                .arg("one\\two")
                .succeeds()
                .stdout_only(format!("{correct}\n"));
        }

        // Tests for a character that forces quotation in shell-style escaping
        // after a character in a dollar expression
        at.touch("one\n&two");
        for (arg, correct) in [
            ("--quoting-style=shell-escape", "'one'$'\\n''&two'"),
            ("--quoting-style=shell-escape-always", "'one'$'\\n''&two'"),
        ] {
            scene
                .ucmd()
                .arg("--hide-control-chars")
                .arg(arg)
                .arg("one\n&two")
                .succeeds()
                .stdout_only(format!("{correct}\n"));
        }
    }

    scene
        .ucmd()
        .arg("one two")
        .succeeds()
        .stdout_only("one two\n");
    // TODO: TTY-expected output
    // .stdout_only("'one two'\n");

    for (arg, correct) in [
        ("--quoting-style=literal", "one two"),
        ("-N", "one two"),
        ("--literal", "one two"),
        ("--l", "one two"),
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
            .arg("--hide-control-chars")
            .arg(arg)
            .arg("one two")
            .succeeds()
            .stdout_only(format!("{correct}\n"));
    }

    scene.ucmd().arg("one").succeeds().stdout_only("one\n");

    for (arg, correct) in [
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
            .arg("--hide-control-chars")
            .arg(arg)
            .arg("one")
            .succeeds()
            .stdout_only(format!("{correct}\n"));
    }
}

#[test]
fn test_ls_quoting_style_env_var_default() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("foo-1"));
    at.touch(at.plus_as_string("bar-2"));

    // If no quoting style argument is provided, the QUOTING_STYLE environment variable
    // shall be used.

    let correct_c = "\"bar-2\"\n\"foo-1\"";
    scene
        .ucmd()
        .env("QUOTING_STYLE", "c")
        .succeeds()
        .stdout_only(format!("{correct_c}\n"));
}

#[test]
fn test_ls_quoting_style_arg_overrides_env_var() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch(at.plus_as_string("foo-1"));
    at.touch(at.plus_as_string("bar-2"));

    // The quoting style given by the env variable should be
    // overridden by any escape style provided by argument.
    for (arg, correct) in [
        ("--quoting-style=literal", "foo-1"),
        ("-N", "foo-1"),
        ("--quoting-style=escape", "foo-1"),
        ("-b", "foo-1"),
        ("--quoting-style=shell-escape", "foo-1"),
        ("--quoting-style=shell-escape-always", "'foo-1'"),
        ("--quoting-style=shell", "foo-1"),
        ("--quoting-style=shell-always", "'foo-1'"),
    ] {
        scene
            .ucmd()
            .env("QUOTING_STYLE", "c")
            .arg("--hide-control-chars")
            .arg(arg)
            .arg("foo-1")
            .succeeds()
            .stdout_only(format!("{correct}\n"));
    }

    // Another loop to check for the C quoting style that is used as a default above.
    for (arg, correct) in [
        ("--quoting-style=c", "\"foo-1\""),
        ("-Q", "\"foo-1\""),
        ("--quote-name", "\"foo-1\""),
    ] {
        scene
            .ucmd()
            .env("QUOTING_STYLE", "literal")
            .arg("--hide-control-chars")
            .arg(arg)
            .arg("foo-1")
            .succeeds()
            .stdout_only(format!("{correct}\n"));
    }
}

#[test]
fn test_ls_quoting_and_color() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("one two");
    scene
        .ucmd()
        .arg("--color")
        .arg("one two")
        .succeeds()
        .stdout_only("one two\n");
    // TODO: TTY-expected output
    // .stdout_only("'one two'\n");
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
        .arg("--hide=*")
        .arg("-1")
        .succeeds()
        .stdout_only("");

    scene
        .ucmd()
        .arg("--ignore=*")
        .arg("-1")
        .succeeds()
        .stdout_only("");

    scene
        .ucmd()
        .arg("--ignore=irrelevant pattern")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--ignore=README*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--hide=README*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--ignore=*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore=*.md")
        .arg("-1")
        .succeeds()
        .stdout_only(".\n..\nsome_other_file\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("--hide=*.md")
        .arg("-1")
        .succeeds()
        .stdout_only(".\n..\nCONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("-A")
        .arg("--ignore=*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("-A")
        .arg("--hide=*.md")
        .arg("-1")
        .succeeds()
        .stdout_only("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    // Stacking multiple patterns
    scene
        .ucmd()
        .arg("--ignore=README*")
        .arg("--ignore=CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("--hide=README*")
        .arg("--ignore=CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    scene
        .ucmd()
        .arg("--hide=README*")
        .arg("--hide=CONTRIBUTING*")
        .arg("-1")
        .succeeds()
        .stdout_only("some_other_file\n");

    // Invalid patterns
    scene
        .ucmd()
        .arg("--ignore=READ[ME")
        .arg("-1")
        .succeeds()
        .stderr_contains("Invalid pattern")
        .stdout_is("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");

    scene
        .ucmd()
        .arg("--hide=READ[ME")
        .arg("-1")
        .succeeds()
        .stderr_contains("Invalid pattern")
        .stdout_is("CONTRIBUTING.md\nREADME.md\nREADMECAREFULLY.md\nsome_other_file\n");
}

#[test]
#[cfg(unix)]
fn test_ls_ignore_backups() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("somefile");
    at.touch("somebackup~");
    at.touch(".somehiddenfile");
    at.touch(".somehiddenbackup~");

    scene.ucmd().arg("-B").succeeds().stdout_is("somefile\n");
    scene
        .ucmd()
        .arg("--ignore-backups")
        .succeeds()
        .stdout_is("somefile\n");

    scene
        .ucmd()
        .arg("-aB")
        .succeeds()
        .stdout_contains(".somehiddenfile")
        .stdout_contains("somefile")
        .stdout_does_not_contain("somebackup")
        .stdout_does_not_contain(".somehiddenbackup~");

    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore-backups")
        .succeeds()
        .stdout_contains(".somehiddenfile")
        .stdout_contains("somefile")
        .stdout_does_not_contain("somebackup")
        .stdout_does_not_contain(".somehiddenbackup~");
}

// This test fails on windows, see details at #3985
#[cfg(not(windows))]
#[test]
fn test_ls_ignore_explicit_period() {
    // In ls ignore patterns, leading periods must be explicitly specified
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    at.touch(".hidden.yml");
    at.touch("regular.yml");

    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore")
        .arg("?hidden.yml")
        .succeeds()
        .stdout_contains(".hidden.yml")
        .stdout_contains("regular.yml");

    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore")
        .arg("*.yml")
        .succeeds()
        .stdout_contains(".hidden.yml")
        .stdout_does_not_contain("regular.yml");

    // Leading period is explicitly specified
    scene
        .ucmd()
        .arg("-a")
        .arg("--ignore")
        .arg(".*.yml")
        .succeeds()
        .stdout_does_not_contain(".hidden.yml")
        .stdout_contains("regular.yml");
}

// This test fails on windows, see details at #3985
#[cfg(not(windows))]
#[test]
fn test_ls_ignore_negation() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    at.touch("apple");
    at.touch("boy");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("[!a]*")
        .succeeds()
        .stdout_contains("apple")
        .stdout_does_not_contain("boy");

    scene
        .ucmd()
        .arg("--ignore")
        .arg("[^a]*")
        .succeeds()
        .stdout_contains("apple")
        .stdout_does_not_contain("boy");
}

#[test]
fn test_ls_directory() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("some_dir");
    at.symlink_dir("some_dir", "sym_dir");

    at.touch(Path::new("some_dir").join("nested_file").to_str().unwrap());

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

    at.touch(Path::new("some_dir").join("nested_file").to_str().unwrap());

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

#[test]
fn test_ls_sort_extension() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    for filename in [
        "file1",
        "file2",
        "anotherFile",
        ".hidden",
        ".file.1",
        ".file.2",
        "file.1",
        "file.2",
        "anotherFile.1",
        "anotherFile.2",
        "file.ext",
        "file.debug",
        "anotherFile.ext",
        "anotherFile.debug",
    ] {
        at.touch(filename);
    }

    let expected = vec![
        ".",
        "..",
        ".hidden",
        "anotherFile",
        "file1",
        "file2",
        ".file.1",
        "anotherFile.1",
        "file.1",
        ".file.2",
        "anotherFile.2",
        "file.2",
        "anotherFile.debug",
        "file.debug",
        "anotherFile.ext",
        "file.ext",
        "", // because of '\n' at the end of the output
    ];

    let result = scene.ucmd().arg("-1aX").run();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected,
    );

    let result = scene.ucmd().arg("-1a").arg("--sort=extension").run();
    assert_eq!(
        result.stdout_str().split('\n').collect::<Vec<_>>(),
        expected,
    );
}

#[test]
fn test_ls_path() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file1 = "file1";
    let file2 = "file2";
    let dir = "dir";
    let path = &format!("{dir}/{file2}");

    at.mkdir(dir);
    at.touch(file1);
    at.touch(path);

    let expected_stdout = &format!("{path}\n");
    scene.ucmd().arg(path).run().stdout_is(expected_stdout);

    let expected_stdout = &format!("./{path}\n");
    scene
        .ucmd()
        .arg(format!("./{path}"))
        .run()
        .stdout_is(expected_stdout);

    let abs_path = format!("{}/{}", at.as_string(), path);
    let expected_stdout = format!("{abs_path}\n");

    scene.ucmd().arg(&abs_path).run().stdout_is(expected_stdout);

    let expected_stdout = format!("{path}\n{file1}\n");
    scene
        .ucmd()
        .arg(file1)
        .arg(path)
        .run()
        .stdout_is(expected_stdout);
}

#[test]
fn test_ls_dangling_symlinks() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("temp_dir");
    at.symlink_file("does_not_exist", "temp_dir/dangle");

    scene
        .ucmd()
        .arg("-L")
        .arg("temp_dir/dangle")
        .fails()
        .code_is(2);
    scene
        .ucmd()
        .arg("-H")
        .arg("temp_dir/dangle")
        .fails()
        .code_is(2);

    scene
        .ucmd()
        .arg("temp_dir/dangle")
        .succeeds()
        .stdout_contains("dangle");

    scene
        .ucmd()
        .arg("-Li")
        .arg("temp_dir")
        .fails()
        .code_is(1)
        .stderr_contains("cannot access")
        .stderr_contains("No such file or directory")
        .stdout_contains(if cfg!(windows) { "dangle" } else { "? dangle" });

    scene
        .ucmd()
        .arg("-LZ")
        .arg("temp_dir")
        .fails()
        .code_is(1)
        .stderr_contains("cannot access")
        .stderr_contains("No such file or directory")
        .stdout_contains(if cfg!(windows) { "dangle" } else { "? dangle" });

    scene
        .ucmd()
        .arg("-Ll")
        .arg("temp_dir")
        .fails()
        .code_is(1)
        .stdout_contains("l?????????");

    #[cfg(unix)]
    {
        // Check padding is the same for real files and dangling links, in non-long formats
        at.touch("temp_dir/real_file");

        let real_file_res = scene.ucmd().arg("-Li1").arg("temp_dir").fails();
        real_file_res.code_is(1);
        let real_file_stdout_len = String::from_utf8(real_file_res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .strip_suffix("real_file")
            .unwrap()
            .len();

        let dangle_file_res = scene.ucmd().arg("-Li1").arg("temp_dir").fails();
        dangle_file_res.code_is(1);
        let dangle_stdout_len = String::from_utf8(dangle_file_res.stdout().to_owned())
            .ok()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .strip_suffix("dangle")
            .unwrap()
            .len();

        assert_eq!(real_file_stdout_len, dangle_stdout_len);
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_ls_context1() {
    use selinux::{self, KernelSupport};
    if selinux::kernel_support() == KernelSupport::Unsupported {
        println!("test skipped: Kernel has no support for SElinux context",);
        return;
    }

    let file = "test_ls_context_file";
    let expected = format!("unconfined_u:object_r:user_tmp_t:s0 {}\n", file);
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch(file);
    ucmd.args(&["-Z", file]).succeeds().stdout_is(expected);
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_ls_context2() {
    use selinux::{self, KernelSupport};
    if selinux::kernel_support() == KernelSupport::Unsupported {
        println!("test skipped: Kernel has no support for SElinux context",);
        return;
    }
    let ts = TestScenario::new(util_name!());
    for c_flag in ["-Z", "--context"] {
        ts.ucmd()
            .args(&[c_flag, "/"])
            .succeeds()
            .stdout_only(unwrap_or_return!(expected_result(&ts, &[c_flag, "/"])).stdout_str());
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn test_ls_context_format() {
    use selinux::{self, KernelSupport};
    if selinux::kernel_support() == KernelSupport::Unsupported {
        println!("test skipped: Kernel has no support for SElinux context",);
        return;
    }
    let ts = TestScenario::new(util_name!());
    // NOTE:
    // --format=long/verbose matches the output of GNU's ls for --context
    // except for the size count which may differ to the size count reported by GNU's ls.
    for word in [
        "across",
        "commas",
        "horizontal",
        // "long",
        "single-column",
        // "verbose",
        "vertical",
    ] {
        let format = format!("--format={}", word);
        ts.ucmd()
            .args(&["-Z", format.as_str(), "/"])
            .succeeds()
            .stdout_only(
                unwrap_or_return!(expected_result(&ts, &["-Z", format.as_str(), "/"])).stdout_str(),
            );
    }
}

#[test]
#[allow(non_snake_case)]
fn test_ls_a_A() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("-A")
        .arg("-a")
        .succeeds()
        .stdout_contains(".")
        .stdout_contains("..");

    scene
        .ucmd()
        .arg("-a")
        .arg("-A")
        .succeeds()
        .stdout_does_not_contain(".")
        .stdout_does_not_contain("..");
}

#[test]
#[allow(non_snake_case)]
fn test_ls_multiple_a_A() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("-a")
        .arg("-a")
        .succeeds()
        .stdout_contains(".")
        .stdout_contains("..");

    scene
        .ucmd()
        .arg("-A")
        .arg("-A")
        .succeeds()
        .stdout_does_not_contain(".")
        .stdout_does_not_contain("..");
}

#[test]
#[cfg(feature = "ln")]
fn test_ls_quoting() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("ln")
        .arg("-s")
        .arg("'need quoting'")
        .arg("symlink")
        .succeeds();
    scene
        .ucmd()
        .arg("-l")
        .arg("--quoting-style=shell-escape")
        .arg("symlink")
        .succeeds()
        .stdout_contains("\'need quoting\'");
}

#[test]
#[cfg(feature = "ln")]
fn test_ls_quoting_color() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("ln")
        .arg("-s")
        .arg("'need quoting'")
        .arg("symlink")
        .succeeds();
    scene
        .ucmd()
        .arg("-l")
        .arg("--quoting-style=shell-escape")
        .arg("--color=auto")
        .arg("symlink")
        .succeeds()
        .stdout_contains("\'need quoting\'");
}

#[test]
fn test_ls_dereference_looped_symlinks_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("loop");
    at.relative_symlink_dir("../loop", "loop/sub");

    ucmd.args(&["-RL", "loop"])
        .fails()
        .code_is(2)
        .stderr_contains("not listing already-listed directory");
}

#[test]
fn test_dereference_dangling_color() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("wat", "nonexistent");
    let out_exp = ucmd.args(&["--color"]).run().stdout_move_str();

    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("wat", "nonexistent");
    ucmd.args(&["-L", "--color"])
        .fails()
        .code_is(1)
        .stderr_contains("No such file or directory")
        .stdout_is(out_exp);
}

#[test]
fn test_dereference_symlink_dir_color() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.mkdir("dir1/link");
    let out_exp = ucmd.args(&["--color", "dir1"]).run().stdout_move_str();

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.mkdir("dir2");
    at.relative_symlink_dir("../dir2", "dir1/link");
    ucmd.args(&["-L", "--color", "dir1"])
        .succeeds()
        .stdout_is(out_exp);
}

#[test]
fn test_dereference_symlink_file_color() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.touch("dir1/link");
    let out_exp = ucmd.args(&["--color", "dir1"]).run().stdout_move_str();

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.touch("file");
    at.relative_symlink_file("../file", "dir1/link");
    ucmd.args(&["-L", "--color", "dir1"])
        .succeeds()
        .stdout_is(out_exp);
}

#[test]
fn test_tabsize_option() {
    let scene = TestScenario::new(util_name!());

    scene.ucmd().args(&["-T", "3"]).succeeds();
    scene.ucmd().args(&["--tabsize", "0"]).succeeds();
    scene.ucmd().arg("-T").fails();
}

#[ignore = "issue #3624"]
#[test]
fn test_tabsize_formatting() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("aaaaaaaa");
    at.touch("bbbb");
    at.touch("cccc");
    at.touch("dddddddd");

    ucmd.args(&["-T", "4"])
        .succeeds()
        .stdout_is("aaaaaaaa bbbb\ncccc\t dddddddd");

    ucmd.args(&["-T", "2"])
        .succeeds()
        .stdout_is("aaaaaaaa bbbb\ncccc\t\t dddddddd");

    // use spaces
    ucmd.args(&["-T", "0"])
        .succeeds()
        .stdout_is("aaaaaaaa bbbb\ncccc     dddddddd");
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "illumos",
    target_os = "solaris"
))]
#[test]
fn test_device_number() {
    use std::fs::{metadata, read_dir};
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    use uucore::libc::{dev_t, major, minor};

    let dev_dir = read_dir("/dev").unwrap();
    // let's use the first device for test
    let blk_dev = dev_dir
        .map(|res_entry| res_entry.unwrap())
        .find(|entry| {
            entry.file_type().unwrap().is_block_device()
                || entry.file_type().unwrap().is_char_device()
        })
        .expect("Expect a block/char device");
    let blk_dev_path = blk_dev.path();
    let blk_dev_meta = metadata(blk_dev_path.as_path()).unwrap();
    let blk_dev_number = blk_dev_meta.rdev() as dev_t;
    let (major, minor) = unsafe { (major(blk_dev_number), minor(blk_dev_number)) };
    let major_minor_str = format!("{}, {}", major, minor);

    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .arg("-l")
        .arg(blk_dev_path.to_str().expect("should be UTF-8 encoded"))
        .succeeds()
        .stdout_contains(major_minor_str);
}

#[test]
#[cfg(target_os = "linux")]
fn test_invalid_utf8() {
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = OsStr::from_bytes(b"-\xE0-foo");
    at.touch(filename);
    ucmd.succeeds();
}

#[cfg(all(unix, feature = "chmod"))]
#[test]
fn test_ls_perm_io_errors() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("d");
    at.symlink_file("/", "d/s");

    scene.ccmd("chmod").arg("600").arg("d").succeeds();

    scene
        .ucmd()
        .arg("-l")
        .arg("d")
        .fails()
        .code_is(1)
        .stderr_contains("Permission denied");
}

#[test]
fn test_ls_dired_incompatible() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("--dired")
        .fails()
        .code_is(1)
        .stderr_contains("--dired requires --format=long");
}

#[test]
fn test_ls_dired_and_zero_are_incompatible() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("--dired")
        .arg("-l")
        .arg("--zero")
        .fails()
        .code_is(2)
        .stderr_contains("--dired and --zero are incompatible");
}

#[test]
fn test_ls_dired_recursive() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("--dired")
        .arg("-l")
        .arg("-R")
        .succeeds()
        .stdout_does_not_contain("//DIRED//")
        .stdout_contains("  total 0")
        .stdout_contains("//SUBDIRED// 2 3")
        .stdout_contains("//DIRED-OPTIONS// --quoting-style");
}

#[test]
fn test_ls_dired_recursive_multiple() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("d");
    at.mkdir("d/d1");
    at.mkdir("d/d2");
    at.touch("d/d2/a");
    at.touch("d/d2/c2");
    at.touch("d/d1/f1");
    at.touch("d/d1/file-long");

    let mut cmd = scene.ucmd();
    cmd.arg("--dired").arg("-l").arg("-R").arg("d");

    let result = cmd.succeeds();

    let output = result.stdout_str().to_string();
    println!("Output:\n{}", output);

    let dired_line = output
        .lines()
        .find(|&line| line.starts_with("//DIRED//"))
        .unwrap();
    let positions: Vec<usize> = dired_line
        .split_whitespace()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();
    println!("Parsed byte positions: {:?}", positions);
    assert_eq!(positions.len() % 2, 0); // Ensure there's an even number of positions

    let filenames: Vec<String> = positions
        .chunks(2)
        .map(|chunk| {
            let start_pos = chunk[0];
            let end_pos = chunk[1];
            let filename = String::from_utf8(output.as_bytes()[start_pos..=end_pos].to_vec())
                .unwrap()
                .trim()
                .to_string();
            println!("Extracted filename: {}", filename);
            filename
        })
        .collect();

    println!("Extracted filenames: {:?}", filenames);
    assert_eq!(filenames, vec!["d1", "d2", "f1", "file-long", "a", "c2"]);
}

#[test]
fn test_ls_dired_simple() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    scene
        .ucmd()
        .arg("--dired")
        .arg("-l")
        .succeeds()
        .stdout_contains("  total 0");

    at.mkdir("d");
    at.touch("d/a1");
    let mut cmd = scene.ucmd();
    cmd.arg("--dired").arg("-l").arg("d");
    let result = cmd.succeeds();
    result.stdout_contains("  total 0");
    println!("    result.stdout = {:#?}", result.stdout_str());

    let dired_line = result
        .stdout_str()
        .lines()
        .find(|&line| line.starts_with("//DIRED//"))
        .unwrap();
    let positions: Vec<usize> = dired_line
        .split_whitespace()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();

    assert_eq!(positions.len(), 2);

    let start_pos = positions[0];
    let end_pos = positions[1];

    // Extract the filename using the positions
    let filename =
        String::from_utf8(result.stdout_str().as_bytes()[start_pos..end_pos].to_vec()).unwrap();

    assert_eq!(filename, "a1");
}

#[test]
fn test_ls_dired_complex() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("d");
    at.mkdir("d/d");
    at.touch("d/a1");
    at.touch("d/a22");
    at.touch("d/a333");
    at.touch("d/a4444");

    let mut cmd = scene.ucmd();
    cmd.arg("--dired").arg("-l").arg("d");
    let result = cmd.succeeds();

    // Number of blocks. We run this test only if the default size of a newly created directory is
    // 4096 bytes to prevent it from failing where this is not the case (e.g. using tmpfs for /tmp).
    #[cfg(target_os = "linux")]
    if at.metadata("d/d").len() == 4096 {
        result.stdout_contains("  total 4");
    }

    let output = result.stdout_str().to_string();
    println!("Output:\n{}", output);

    let dired_line = output
        .lines()
        .find(|&line| line.starts_with("//DIRED//"))
        .unwrap();
    let positions: Vec<usize> = dired_line
        .split_whitespace()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();
    println!("{:?}", positions);
    println!("Parsed byte positions: {:?}", positions);
    assert_eq!(positions.len() % 2, 0); // Ensure there's an even number of positions

    let filenames: Vec<String> = positions
        .chunks(2)
        .map(|chunk| {
            let start_pos = chunk[0];
            let end_pos = chunk[1];
            let filename = String::from_utf8(output.as_bytes()[start_pos..=end_pos].to_vec())
                .unwrap()
                .trim()
                .to_string();
            println!("Extracted filename: {}", filename);
            filename
        })
        .collect();

    println!("Extracted filenames: {:?}", filenames);
    assert_eq!(filenames, vec!["a1", "a22", "a333", "a4444", "d"]);
}

#[test]
fn test_ls_subdired_complex() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("dir1");
    at.mkdir("dir1/d");
    at.mkdir("dir1/c2");
    at.touch("dir1/a1");
    at.touch("dir1/a22");
    at.touch("dir1/a333");
    at.touch("dir1/c2/a4444");

    let mut cmd = scene.ucmd();
    cmd.arg("--dired").arg("-l").arg("-R").arg("dir1");
    let result = cmd.succeeds();

    let output = result.stdout_str().to_string();
    println!("Output:\n{}", output);

    let dired_line = output
        .lines()
        .find(|&line| line.starts_with("//SUBDIRED//"))
        .unwrap();
    let positions: Vec<usize> = dired_line
        .split_whitespace()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();
    println!("Parsed byte positions: {:?}", positions);
    assert_eq!(positions.len() % 2, 0); // Ensure there's an even number of positions

    let dirnames: Vec<String> = positions
        .chunks(2)
        .map(|chunk| {
            let start_pos = chunk[0];
            let end_pos = chunk[1];
            let dirname =
                String::from_utf8(output.as_bytes()[start_pos..end_pos].to_vec()).unwrap();
            println!("Extracted dirname: {}", dirname);
            dirname
        })
        .collect();

    println!("Extracted dirnames: {:?}", dirnames);
    #[cfg(unix)]
    assert_eq!(dirnames, vec!["dir1", "dir1/c2", "dir1/d"]);
    #[cfg(windows)]
    assert_eq!(dirnames, vec!["dir1", "dir1\\c2", "dir1\\d"]);
}

#[ignore = "issue #5396"]
#[test]
fn test_ls_cf_output_should_be_delimited_by_tab() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("e");
    at.mkdir("e/a2345");
    at.mkdir("e/b");

    ucmd.args(&["-CF", "e"])
        .succeeds()
        .stdout_is("a2345/\tb/\n");
}

#[cfg(all(unix, feature = "dd"))]
#[test]
fn test_posixly_correct_and_block_size_env_vars() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("dd")
        .arg("if=/dev/zero")
        .arg("of=file")
        .arg("bs=1024")
        .arg("count=1")
        .succeeds();

    scene
        .ucmd()
        .arg("-l")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1024 ");

    scene
        .ucmd()
        .arg("-l")
        .env("POSIXLY_CORRECT", "some_value")
        .succeeds()
        .stdout_contains_line("total 8")
        .stdout_contains(" 1024 ");

    scene
        .ucmd()
        .arg("-l")
        .env("LS_BLOCK_SIZE", "512")
        .succeeds()
        .stdout_contains_line("total 8")
        .stdout_contains(" 2 ");

    scene
        .ucmd()
        .arg("-l")
        .env("BLOCK_SIZE", "512")
        .succeeds()
        .stdout_contains_line("total 8")
        .stdout_contains(" 2 ");

    scene
        .ucmd()
        .arg("-l")
        .env("BLOCKSIZE", "512")
        .succeeds()
        .stdout_contains_line("total 8")
        .stdout_contains(" 1024 ");
}

#[cfg(all(unix, feature = "dd"))]
#[test]
fn test_posixly_correct_and_block_size_env_vars_with_k() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("dd")
        .arg("if=/dev/zero")
        .arg("of=file")
        .arg("bs=1024")
        .arg("count=1")
        .succeeds();

    scene
        .ucmd()
        .arg("-l")
        .arg("-k")
        .env("POSIXLY_CORRECT", "some_value")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1024 ");

    scene
        .ucmd()
        .arg("-l")
        .arg("-k")
        .env("LS_BLOCK_SIZE", "512")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 2 ");

    scene
        .ucmd()
        .arg("-l")
        .arg("-k")
        .env("BLOCK_SIZE", "512")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 2 ");

    scene
        .ucmd()
        .arg("-l")
        .arg("-k")
        .env("BLOCKSIZE", "512")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1024 ");
}

#[test]
fn test_ls_invalid_block_size() {
    new_ucmd!()
        .arg("--block-size=invalid")
        .fails()
        .code_is(2)
        .no_stdout()
        .stderr_is("ls: invalid --block-size argument 'invalid'\n");
}

#[cfg(all(unix, feature = "dd"))]
#[test]
fn test_ls_invalid_block_size_in_env_var() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("dd")
        .arg("if=/dev/zero")
        .arg("of=file")
        .arg("bs=1024")
        .arg("count=1")
        .succeeds();

    scene
        .ucmd()
        .arg("-og")
        .env("LS_BLOCK_SIZE", "invalid")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1 1 "); // hardlink count + file size

    scene
        .ucmd()
        .arg("-og")
        .env("BLOCK_SIZE", "invalid")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1 1 "); // hardlink count + file size

    scene
        .ucmd()
        .arg("-og")
        .env("BLOCKSIZE", "invalid")
        .succeeds()
        .stdout_contains_line("total 4")
        .stdout_contains(" 1024 ");
}

#[cfg(all(unix, feature = "dd"))]
#[test]
fn test_ls_block_size_override() {
    let scene = TestScenario::new(util_name!());

    scene
        .ccmd("dd")
        .arg("if=/dev/zero")
        .arg("of=file")
        .arg("bs=1024")
        .arg("count=1")
        .succeeds();

    // --si "wins"
    scene
        .ucmd()
        .arg("-s")
        .arg("--block-size=512")
        .arg("--si")
        .succeeds()
        .stdout_contains_line("total 4.1k");

    // --block-size "wins"
    scene
        .ucmd()
        .arg("-s")
        .arg("--si")
        .arg("--block-size=512")
        .succeeds()
        .stdout_contains_line("total 8");

    // --human-readable "wins"
    scene
        .ucmd()
        .arg("-s")
        .arg("--block-size=512")
        .arg("--human-readable")
        .succeeds()
        .stdout_contains_line("total 4.0K");

    // --block-size "wins"
    scene
        .ucmd()
        .arg("-s")
        .arg("--human-readable")
        .arg("--block-size=512")
        .succeeds()
        .stdout_contains_line("total 8");
}

#[test]
fn test_ls_block_size_override_self() {
    new_ucmd!()
        .arg("--block-size=512")
        .arg("--block-size=512")
        .succeeds();

    new_ucmd!()
        .arg("--human-readable")
        .arg("--human-readable")
        .succeeds();

    new_ucmd!().arg("--si").arg("--si").succeeds();
}

#[test]
fn test_ls_hyperlink() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "a.txt";

    at.touch(file);

    let path = at.root_dir_resolved();
    let separator = std::path::MAIN_SEPARATOR_STR;

    let result = scene.ucmd().arg("--hyperlink").succeeds();
    assert!(result.stdout_str().contains("\x1b]8;;file://"));
    assert!(result
        .stdout_str()
        .contains(&format!("{path}{separator}{file}\x07{file}\x1b]8;;\x07")));

    let result = scene.ucmd().arg("--hyperlink=always").succeeds();
    assert!(result.stdout_str().contains("\x1b]8;;file://"));
    assert!(result
        .stdout_str()
        .contains(&format!("{path}{separator}{file}\x07{file}\x1b]8;;\x07")));

    for argument in [
        "--hyperlink=never",
        "--hyperlink=neve", // spell-checker:disable-line
        "--hyperlink=ne",   // spell-checker:disable-line
        "--hyperlink=n",
    ] {
        scene
            .ucmd()
            .arg(argument)
            .succeeds()
            .stdout_is(format!("{file}\n"));
    }
}

// spell-checker: disable
#[test]
fn test_ls_hyperlink_encode_link() {
    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(not(target_os = "windows"))]
    {
        at.touch("back\\slash");
        at.touch("ques?tion");
    }
    at.touch("encoded%3Fquestion");
    at.touch("sp ace");

    let result = ucmd.arg("--hyperlink").succeeds();
    #[cfg(not(target_os = "windows"))]
    {
        assert!(result
            .stdout_str()
            .contains("back%5cslash\x07back\\slash\x1b]8;;\x07"));
        assert!(result
            .stdout_str()
            .contains("ques%3ftion\x07ques?tion\x1b]8;;\x07"));
    }
    assert!(result
        .stdout_str()
        .contains("encoded%253Fquestion\x07encoded%3Fquestion\x1b]8;;\x07"));
    assert!(result
        .stdout_str()
        .contains("sp%20ace\x07sp ace\x1b]8;;\x07"));
}
// spell-checker: enable

#[test]
fn test_ls_hyperlink_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir_a = "a";
    let dir_b = "b";

    at.mkdir(dir_a);
    at.mkdir(dir_b);

    let path = at.root_dir_resolved();
    let separator = std::path::MAIN_SEPARATOR_STR;

    let result = scene
        .ucmd()
        .arg("--hyperlink")
        .arg(dir_a)
        .arg(dir_b)
        .succeeds();

    assert!(result.stdout_str().contains("\x1b]8;;file://"));
    assert!(result
        .stdout_str()
        .lines()
        .next()
        .unwrap()
        .contains(&format!("{path}{separator}{dir_a}\x07{dir_a}\x1b]8;;\x07:")));
    assert_eq!(result.stdout_str().lines().nth(1).unwrap(), "");
    assert!(result
        .stdout_str()
        .lines()
        .nth(2)
        .unwrap()
        .contains(&format!("{path}{separator}{dir_b}\x07{dir_b}\x1b]8;;\x07:")));
}

#[test]
fn test_ls_color_do_not_reset() {
    let scene: TestScenario = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("example");
    at.mkdir("example/a");
    at.mkdir("example/b");

    let result = scene
        .ucmd()
        .arg("--color=always")
        .arg("example/")
        .succeeds();
    // the second color code should not have a reset
    assert_eq!(
        result.stdout_str().escape_default().to_string(),
        "\\u{1b}[0m\\u{1b}[01;34ma\\u{1b}[0m\\n\\u{1b}[01;34mb\\u{1b}[0m\\n"
    );
}

#[cfg(all(unix, feature = "chmod"))]
#[test]
fn test_term_colorterm() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("exe");
    scene.ccmd("chmod").arg("+x").arg("exe").succeeds();

    // Should show colors
    let result = scene
        .ucmd()
        .arg("--color=always")
        .env("LS_COLORS", "")
        .env("TERM", "")
        .succeeds();
    assert_eq!(
        result.stdout_str().trim().escape_default().to_string(),
        "\\u{1b}[0m\\u{1b}[01;32mexe\\u{1b}[0m"
    );

    // Should show colors
    let result = scene
        .ucmd()
        .arg("--color=always")
        .env("LS_COLORS", "")
        .env("COLORTERM", "")
        .succeeds();
    assert_eq!(
        result.stdout_str().trim().escape_default().to_string(),
        "\\u{1b}[0m\\u{1b}[01;32mexe\\u{1b}[0m"
    );

    // No colors
    let result = scene
        .ucmd()
        .arg("--color=always")
        .env("LS_COLORS", "")
        .env("TERM", "")
        .env("COLORTERM", "")
        .succeeds();
    assert_eq!(
        result.stdout_str().trim().escape_default().to_string(),
        "exe"
    );

    // No colors
    let result = scene
        .ucmd()
        .arg("--color=always")
        .env("LS_COLORS", "")
        .env("TERM", "dumb")
        .env("COLORTERM", "")
        .succeeds();
    assert_eq!(
        result.stdout_str().trim().escape_default().to_string(),
        "exe"
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn test_acl_display() {
    use std::process::Command;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let path = "a42";
    at.mkdir(path);

    let path = at.plus_as_string(path);
    // calling the command directly. xattr requires some dev packages to be installed
    // and it adds a complex dependency just for a test
    match Command::new("setfacl")
        .args(["-d", "-m", "group::rwx", &path])
        .status()
        .map(|status| status.code())
    {
        Ok(Some(0)) => {}
        Ok(_) => {
            println!("test skipped: setfacl failed");
            return;
        }
        Err(e) => {
            println!("test skipped: setfacl failed with {}", e);
            return;
        }
    }

    scene
        .ucmd()
        .args(&["-lda", &path])
        .succeeds()
        .stdout_contains("+");
}
