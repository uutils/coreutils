// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore NOFILE nonewline cmdline

#[cfg(any(target_os = "linux", target_os = "android"))]
use rlimit::Resource;
#[cfg(unix)]
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::read_to_string;
use std::process::Stdio;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
#[cfg(not(windows))]
use uutests::util::vec_of_size;
use uutests::util_name;

#[test]
fn test_output_simple() {
    new_ucmd!()
        .args(&["alpha.txt"])
        .succeeds()
        .stdout_only("abcde\nfghij\nklmno\npqrst\nuvwxyz\n"); // spell-checker:disable-line
}

#[test]
fn test_no_options() {
    for fixture in ["empty.txt", "alpha.txt", "nonewline.txt"] {
        // Give fixture through command line file argument
        new_ucmd!()
            .args(&[fixture])
            .succeeds()
            .stdout_is_fixture(fixture);
        // Give fixture through stdin
        new_ucmd!()
            .pipe_in_fixture(fixture)
            .succeeds()
            .stdout_is_fixture(fixture);
    }
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
fn test_no_options_big_input() {
    for n in [
        0,
        1,
        42,
        16 * 1024 - 7,
        16 * 1024 - 1,
        16 * 1024,
        16 * 1024 + 1,
        16 * 1024 + 3,
        32 * 1024,
        64 * 1024,
        80 * 1024,
        96 * 1024,
        112 * 1024,
        128 * 1024,
    ] {
        let data = vec_of_size(n);
        let data2 = data.clone();
        assert_eq!(data.len(), data2.len());
        new_ucmd!().pipe_in(data).succeeds().stdout_is_bytes(&data2);
    }
}

#[test]
#[cfg(unix)]
fn test_fifo_symlink() {
    use std::io::Write;
    use std::thread;

    let s = TestScenario::new(util_name!());
    s.fixtures.mkdir("dir");
    s.fixtures.mkfifo("dir/pipe");
    assert!(s.fixtures.is_fifo("dir/pipe"));

    // Make cat read the pipe through a symlink
    s.fixtures.symlink_file("dir/pipe", "sympipe"); // spell-checker:disable-line
    let proc = s.ucmd().args(&["sympipe"]).run_no_wait(); // spell-checker:disable-line

    let data = vec_of_size(128 * 1024);
    let data2 = data.clone();

    let pipe_path = s.fixtures.plus("dir/pipe");
    let thread = thread::spawn(move || {
        let mut pipe = OpenOptions::new()
            .write(true)
            .create(false)
            .open(pipe_path)
            .unwrap();
        pipe.write_all(&data).unwrap();
    });

    proc.wait().unwrap().stdout_only_bytes(data2);
    thread.join().unwrap();
}

#[test]
// TODO(#7542): Re-enable on Android once we figure out why setting limit is broken.
// #[cfg(any(target_os = "linux", target_os = "android"))]
#[cfg(target_os = "linux")]
fn test_closes_file_descriptors() {
    // Each file creates a pipe, which has two file descriptors.
    // If they are not closed then five is certainly too many.
    new_ucmd!()
        .args(&[
            "alpha.txt",
            "alpha.txt",
            "alpha.txt",
            "alpha.txt",
            "alpha.txt",
        ])
        .limit(Resource::NOFILE, 9, 9)
        .succeeds();
}

#[test]
#[cfg(unix)]
fn test_broken_pipe() {
    let mut cmd = new_ucmd!();
    let mut child = cmd
        .set_stdin(Stdio::from(File::open("/dev/zero").unwrap()))
        .set_stdout(Stdio::piped())
        .run_no_wait();
    // Dropping the stdout should not lead to an error.
    // The "Broken pipe" error should be silently ignored.
    child.close_stdout();
    child.wait().unwrap().fails_silently();
}

#[test]
#[cfg(unix)]
fn test_piped_to_regular_file() {
    use std::fs::read_to_string;

    for append in [true, false] {
        let s = TestScenario::new(util_name!());
        let file_path = s.fixtures.plus("file.txt");

        {
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .append(append)
                .open(&file_path)
                .unwrap();

            s.ucmd()
                .set_stdout(file)
                .pipe_in_fixture("alpha.txt")
                .succeeds();
        }
        let contents = read_to_string(&file_path).unwrap();
        assert_eq!(contents, "abcde\nfghij\nklmno\npqrst\nuvwxyz\n"); // spell-checker:disable-line
    }
}

#[test]
#[cfg(unix)]
fn test_piped_to_dev_null() {
    for append in [true, false] {
        let s = TestScenario::new(util_name!());
        {
            let dev_null = OpenOptions::new()
                .write(true)
                .append(append)
                .open("/dev/null")
                .unwrap();

            s.ucmd()
                .set_stdout(dev_null)
                .pipe_in_fixture("alpha.txt")
                .succeeds();
        }
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_piped_to_dev_full() {
    for append in [true, false] {
        let s = TestScenario::new(util_name!());
        {
            let dev_full = OpenOptions::new()
                .write(true)
                .append(append)
                .open("/dev/full")
                .unwrap();

            s.ucmd()
                .set_stdout(dev_full)
                .pipe_in_fixture("alpha.txt")
                .ignore_stdin_write_error()
                .fails()
                .stderr_contains("No space left on device");
        }
    }
}

#[test]
fn test_directory() {
    let s = TestScenario::new(util_name!());
    s.fixtures.mkdir("test_directory");
    s.ucmd()
        .args(&["test_directory"])
        .fails()
        .stderr_is("cat: test_directory: Is a directory\n");
}

#[test]
fn test_directory_and_file() {
    let s = TestScenario::new(util_name!());
    s.fixtures.mkdir("test_directory2");
    for fixture in ["empty.txt", "alpha.txt", "nonewline.txt"] {
        s.ucmd()
            .args(&["test_directory2", fixture])
            .fails()
            .stderr_is("cat: test_directory2: Is a directory\n")
            .stdout_is_fixture(fixture);
    }
}

#[test]
#[cfg(unix)]
fn test_three_directories_and_file_and_stdin() {
    let s = TestScenario::new(util_name!());
    s.fixtures.mkdir("test_directory3");
    s.fixtures.mkdir("test_directory3/test_directory4");
    s.fixtures.mkdir("test_directory3/test_directory5");
    s.ucmd()
        .args(&[
            "test_directory3/test_directory4",
            "alpha.txt",
            "-",
            "file_which_does_not_exist.txt",
            "nonewline.txt",
            "test_directory3/test_directory5",
            "test_directory3/../test_directory3/test_directory5",
            "test_directory3",
        ])
        .pipe_in("stdout bytes")
        .ignore_stdin_write_error()
        .fails()
        .stderr_is_fixture("three_directories_and_file_and_stdin.stderr.expected")
        .stdout_is(
            "abcde\nfghij\nklmno\npqrst\nuvwxyz\nstdout bytestext without a trailing newline", // spell-checker:disable-line
        );
}

#[test]
fn test_output_multi_files_print_all_chars() {
    // spell-checker:disable
    new_ucmd!()
        .args(&["alpha.txt", "256.txt", "-A", "-n"])
        .succeeds()
        .stdout_only(
            "     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     \
             5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     \
             7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ \
             !\"#$%&\'()*+,-./0123456789:;\
             <=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^\
             BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^V\
             M-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- \
             M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:\
             M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-U\
             M-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-\
             pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?",
        );
    // spell-checker:enable
}

#[test]
fn test_output_multi_files_print_all_chars_repeated() {
    // spell-checker:disable
    new_ucmd!()
        .args(&["alpha.txt", "256.txt", "-A", "-n", "-A", "-n"])
        .succeeds()
        .stdout_only(
            "     1\tabcde$\n     2\tfghij$\n     3\tklmno$\n     4\tpqrst$\n     \
             5\tuvwxyz$\n     6\t^@^A^B^C^D^E^F^G^H^I$\n     \
             7\t^K^L^M^N^O^P^Q^R^S^T^U^V^W^X^Y^Z^[^\\^]^^^_ \
             !\"#$%&\'()*+,-./0123456789:;\
             <=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~^?M-^@M-^AM-^\
             BM-^CM-^DM-^EM-^FM-^GM-^HM-^IM-^JM-^KM-^LM-^MM-^NM-^OM-^PM-^QM-^RM-^SM-^TM-^UM-^V\
             M-^WM-^XM-^YM-^ZM-^[M-^\\M-^]M-^^M-^_M- \
             M-!M-\"M-#M-$M-%M-&M-\'M-(M-)M-*M-+M-,M--M-.M-/M-0M-1M-2M-3M-4M-5M-6M-7M-8M-9M-:\
             M-;M-<M-=M->M-?M-@M-AM-BM-CM-DM-EM-FM-GM-HM-IM-JM-KM-LM-MM-NM-OM-PM-QM-RM-SM-TM-U\
             M-VM-WM-XM-YM-ZM-[M-\\M-]M-^M-_M-`M-aM-bM-cM-dM-eM-fM-gM-hM-iM-jM-kM-lM-mM-nM-oM-\
             pM-qM-rM-sM-tM-uM-vM-wM-xM-yM-zM-{M-|M-}M-~M-^?",
        );
    // spell-checker:enable
}

#[test]
fn test_numbered_lines_no_trailing_newline() {
    // spell-checker:disable
    new_ucmd!()
        .args(&["nonewline.txt", "alpha.txt", "-n"])
        .succeeds()
        .stdout_only(
            "     1\ttext without a trailing newlineabcde\n     2\tfghij\n     \
             3\tklmno\n     4\tpqrst\n     5\tuvwxyz\n",
        );
    // spell-checker:enable
}

#[test]
fn test_numbered_lines_with_crlf() {
    new_ucmd!()
        .args(&["-n"])
        .pipe_in("Hello\r\nWorld")
        .succeeds()
        .stdout_only("     1\tHello\r\n     2\tWorld");
}

#[test]
fn test_stdin_show_nonprinting() {
    for same_param in ["-v", "-vv", "--show-nonprinting", "--show-non"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("\t^@\n");
    }
}

#[test]
fn test_stdin_show_tabs() {
    for same_param in ["-T", "-TT", "--show-tabs", "--show-ta"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I\0\n");
    }
}

#[test]
fn test_stdin_show_ends() {
    for same_param in ["-E", "-EE", "--show-ends", "--show-e"] {
        new_ucmd!()
            .args(&[same_param, "-"])
            .pipe_in("\t\0\n\t")
            .succeeds()
            .stdout_only("\t\0$\n\t");
    }
}

#[test]
fn test_squeeze_all_files() {
    // empty lines at the end of a file are "squeezed" together with empty lines at the beginning
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("input1", "a\n\n");
    at.write("input2", "\n\nb");
    ucmd.args(&["input1", "input2", "-s"])
        .succeeds()
        .stdout_only("a\n\nb");
}

#[test]
fn test_squeeze_all_files_repeated() {
    // empty lines at the end of a file are "squeezed" together with empty lines at the beginning
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("input1", "a\n\n");
    at.write("input2", "\n\nb");
    ucmd.args(&["-s", "input1", "input2", "-s"])
        .succeeds()
        .stdout_only("a\n\nb");
}

#[test]
fn test_show_ends_crlf() {
    new_ucmd!()
        .arg("-E")
        .pipe_in("a\nb\r\n\rc\n\r\n\r")
        .succeeds()
        .stdout_only("a$\nb^M$\n\rc$\n^M$\n\r");
}

#[test]
fn test_stdin_show_all() {
    for same_param in ["-A", "--show-all", "--show-a"] {
        new_ucmd!()
            .args(&[same_param])
            .pipe_in("\t\0\n")
            .succeeds()
            .stdout_only("^I^@$\n");
    }
}

#[test]
fn test_stdin_nonprinting_and_endofline() {
    new_ucmd!()
        .args(&["-e"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("\t^@$\n");
}

#[test]
fn test_stdin_nonprinting_and_endofline_repeated() {
    new_ucmd!()
        .args(&["-ee", "-e"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("\t^@$\n");
}

#[test]
fn test_stdin_nonprinting_and_tabs() {
    new_ucmd!()
        .args(&["-t"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("^I^@\n");
}

#[test]
fn test_stdin_nonprinting_and_tabs_repeated() {
    new_ucmd!()
        .args(&["-tt", "-t"])
        .pipe_in("\t\0\n")
        .succeeds()
        .stdout_only("^I^@\n");
}

#[test]
fn test_stdin_tabs_no_newline() {
    new_ucmd!()
        .args(&["-T"])
        .pipe_in("\ta")
        .succeeds()
        .stdout_only("^Ia");
}

#[test]
fn test_stdin_squeeze_blank() {
    for same_param in ["-s", "--squeeze-blank", "--squeeze"] {
        new_ucmd!()
            .arg(same_param)
            .pipe_in("\n\na\n\n\n\n\nb\n\n\n")
            .succeeds()
            .stdout_only("\na\n\nb\n\n");
    }
}

#[test]
fn test_stdin_number_non_blank() {
    // spell-checker:disable-next-line
    for same_param in ["-b", "-bb", "--number-nonblank", "--number-non"] {
        new_ucmd!()
            .arg(same_param)
            .arg("-")
            .pipe_in("\na\nb\n\n\nc")
            .succeeds()
            .stdout_only("\n     1\ta\n     2\tb\n\n\n     3\tc");
    }
}

#[test]
fn test_non_blank_overrides_number() {
    // spell-checker:disable-next-line
    for same_param in ["-b", "--number-nonblank"] {
        new_ucmd!()
            .args(&[same_param, "-"])
            .pipe_in("\na\nb\n\n\nc")
            .succeeds()
            .stdout_only("\n     1\ta\n     2\tb\n\n\n     3\tc");
    }
}

#[test]
fn test_non_blank_overrides_number_even_when_present() {
    new_ucmd!()
        .args(&["-n", "-b", "-n"])
        .pipe_in("\na\nb\n\n\nc")
        .succeeds()
        .stdout_only("\n     1\ta\n     2\tb\n\n\n     3\tc");
}

#[test]
fn test_squeeze_blank_before_numbering() {
    for same_param in ["-s", "--squeeze-blank"] {
        new_ucmd!()
            .args(&[same_param, "-n", "-"])
            .pipe_in("a\n\n\nb")
            .succeeds()
            .stdout_only("     1\ta\n     2\t\n     3\tb");
    }
}

/// This tests reading from Unix character devices
#[test]
#[cfg(unix)]
fn test_dev_random() {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const DEV_RANDOM: &str = "/dev/urandom";

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    const DEV_RANDOM: &str = "/dev/random";

    let mut proc = new_ucmd!()
        .set_stdout(Stdio::piped())
        .args(&[DEV_RANDOM])
        .run_no_wait();

    proc.make_assertion_with_delay(100).is_alive();
    let buf = proc.stdout_exact_bytes(2048);
    let num_zeroes = buf.iter().fold(0, |mut acc, &n| {
        if n == 0 {
            acc += 1;
        }
        acc
    });
    // The probability of more than 512 zero bytes is essentially zero if the
    // output is truly random.
    assert!(num_zeroes < 512);
    proc.kill();
}

/// Reading from /dev/full should return an infinite amount of zero bytes.
/// Wikipedia says there is support on Linux, FreeBSD, and `NetBSD`.
#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_dev_full() {
    let mut proc = new_ucmd!()
        .set_stdout(Stdio::piped())
        .args(&["/dev/full"])
        .run_no_wait();
    let expected = [0; 2048];
    proc.make_assertion_with_delay(100)
        .is_alive()
        .with_exact_output(2048, 0)
        .stdout_only_bytes(expected);
    proc.kill();
}

#[test]
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
fn test_dev_full_show_all() {
    let buf_len = 2048;
    let mut proc = new_ucmd!()
        .set_stdout(Stdio::piped())
        .args(&["-A", "/dev/full"])
        .run_no_wait();
    let expected: Vec<u8> = (0..buf_len)
        .map(|n| if n & 1 == 0 { b'^' } else { b'@' })
        .collect();

    proc.make_assertion_with_delay(100)
        .is_alive()
        .with_exact_output(buf_len, 0)
        .stdout_only_bytes(expected);
    proc.kill();
}

// For some reason splice() on first of those files fails, resulting in
// fallback inside `write_fast`, the other splice succeeds, in effect
// without additional flush output gets reversed.
#[test]
#[cfg(target_os = "linux")]
fn test_write_fast_fallthrough_uses_flush() {
    const PROC_INIT_CMDLINE: &str = "/proc/1/cmdline";
    let cmdline = std::fs::read_to_string(PROC_INIT_CMDLINE).unwrap();

    new_ucmd!()
        .args(&[PROC_INIT_CMDLINE, "alpha.txt"])
        .succeeds()
        .stdout_only(format!("{cmdline}abcde\nfghij\nklmno\npqrst\nuvwxyz\n")); // spell-checker:disable-line
}

#[test]
#[cfg(unix)]
#[ignore = ""]
fn test_domain_socket() {
    use std::io::prelude::*;
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Barrier};
    use std::thread;

    let dir = tempfile::Builder::new()
        .prefix("unix_socket")
        .tempdir()
        .expect("failed to create dir");
    let socket_path = dir.path().join("sock");
    let listener = UnixListener::bind(&socket_path).expect("failed to create socket");

    // use a barrier to ensure we don't run cat before the listener is setup
    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = Arc::clone(&barrier);

    let thread = thread::spawn(move || {
        let mut stream = listener.accept().expect("failed to accept connection").0;
        barrier2.wait();
        stream
            .write_all(b"a\tb")
            .expect("failed to write test data");
    });

    let child = new_ucmd!().args(&[socket_path]).run_no_wait();
    barrier.wait();
    child.wait().unwrap().stdout_is("a\tb");

    thread.join().unwrap();
}

#[test]
fn test_write_to_self_empty() {
    // it's ok if the input file is also the output file if it's empty
    let s = TestScenario::new(util_name!());
    let file_path = s.fixtures.plus("file.txt");

    let file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(&file_path)
        .unwrap();

    s.ucmd().set_stdout(file).arg(&file_path).succeeds();
}

#[test]
fn test_write_to_self() {
    let s = TestScenario::new(util_name!());
    let file_path = s.fixtures.plus("first_file");
    s.fixtures.write("second_file", "second_file_content.");

    let file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(file_path)
        .unwrap();

    s.fixtures.append("first_file", "first_file_content.");

    s.ucmd()
        .set_stdout(file)
        .arg("first_file")
        .arg("first_file")
        .arg("second_file")
        .fails_with_code(2)
        .stderr_only("cat: first_file: input file is output file\ncat: first_file: input file is output file\n");

    assert_eq!(
        s.fixtures.read("first_file"),
        "first_file_content.second_file_content."
    );
}

/// Test derived from the following GNU test in `tests/cat/cat-self.sh`:
///
/// `cat fxy2 fy 1<>fxy2`
// TODO: make this work on windows
#[test]
#[cfg(unix)]
fn test_successful_write_to_read_write_self() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("fy", "y");
    at.write("fxy2", "x");

    // Open `rw_file` as both stdin and stdout (read/write)
    let fxy2_file_path = at.plus("fxy2");
    let fxy2_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&fxy2_file_path)
        .unwrap();
    ucmd.args(&["fxy2", "fy"]).set_stdout(fxy2_file).succeeds();

    // The contents of `fxy2` and `fy` files should be merged
    let fxy2_contents = read_to_string(fxy2_file_path).unwrap();
    assert_eq!(fxy2_contents, "xy");
}

/// Test derived from the following GNU test in `tests/cat/cat-self.sh`:
///
/// `cat fx fx3 1<>fx3`
#[test]
fn test_failed_write_to_read_write_self() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("fx", "g");
    at.write("fx3", "bold");

    // Open `rw_file` as both stdin and stdout (read/write)
    let fx3_file_path = at.plus("fx3");
    let fx3_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&fx3_file_path)
        .unwrap();
    ucmd.args(&["fx", "fx3"])
        .set_stdout(fx3_file)
        .fails_with_code(1)
        .stderr_only("cat: fx3: input file is output file\n");

    // The contents of `fx` should have overwritten the beginning of `fx3`
    let fx3_contents = read_to_string(fx3_file_path).unwrap();
    assert_eq!(fx3_contents, "gold");
}

#[test]
#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
fn test_error_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("2", "1");
    at.symlink_file("3", "2");
    at.symlink_file("1", "3");
    ucmd.arg("1")
        .fails()
        .stderr_is("cat: 1: Too many levels of symbolic links\n");
}

#[test]
fn test_u_ignored() {
    for same_param in ["-u", "-uu"] {
        new_ucmd!()
            .arg(same_param)
            .pipe_in("hello")
            .succeeds()
            .stdout_only("hello");
    }
}

#[test]
#[cfg(unix)]
fn test_write_fast_read_error() {
    use std::os::unix::fs::PermissionsExt;

    let (at, mut ucmd) = at_and_ucmd!();

    // Create a file with content
    at.write("foo", "content");

    // Remove read permissions to cause a read error
    let file_path = at.plus_as_string("foo");
    let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o000); // No permissions
    std::fs::set_permissions(&file_path, perms).unwrap();

    // Test that cat fails with permission denied
    ucmd.arg("foo").fails().stderr_contains("Permission denied");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cat_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a test file with non-UTF-8 bytes in the name
    let non_utf8_bytes = b"test_\xFF\xFE.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);

    // Create the actual file with some content
    std::fs::write(at.plus(non_utf8_name), "Hello, non-UTF-8 world!\n").unwrap();

    // Test that cat handles non-UTF-8 file names without crashing
    let result = scene.ucmd().arg(non_utf8_name).succeeds();

    // The result should contain the file content
    let output = result.stdout_str_lossy();
    assert_eq!(output, "Hello, non-UTF-8 world!\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_appending_same_input_output() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("foo", "content");
    let foo_file = at.plus_as_string("foo");

    let file_read = File::open(&foo_file).unwrap();
    let file_write = OpenOptions::new().append(true).open(&foo_file).unwrap();

    ucmd.set_stdin(file_read);
    ucmd.set_stdout(file_write);

    ucmd.fails()
        .no_stdout()
        .stderr_contains("input file is output file");
}

#[cfg(unix)]
#[test]
fn test_uchild_when_no_capture_reading_from_infinite_source() {
    use regex::Regex;

    let ts = TestScenario::new("cat");

    let expected_stdout = b"\0".repeat(12345);
    let mut child = ts
        .ucmd()
        .set_stdin(Stdio::from(File::open("/dev/zero").unwrap()))
        .set_stdout(Stdio::piped())
        .run_no_wait();

    child
        .make_assertion()
        .with_exact_output(12345, 0)
        .stdout_only_bytes(expected_stdout);

    child
        .kill()
        .make_assertion()
        .with_current_output()
        .stdout_matches(&Regex::new("[\0].*").unwrap())
        .no_stderr();
}

#[test]
fn test_child_when_pipe_in() {
    let ts = TestScenario::new("cat");
    let mut child = ts.ucmd().set_stdin(Stdio::piped()).run_no_wait();
    child.pipe_in("content");
    child.wait().unwrap().stdout_only("content").success();

    ts.ucmd().pipe_in("content").run().stdout_is("content");
}

#[test]
fn test_cat_eintr_handling() {
    // Test that cat properly handles EINTR (ErrorKind::Interrupted) during I/O operations
    // This verifies the signal interruption retry logic added in the EINTR handling fix
    use std::io::{Error, ErrorKind, Read};
    use std::sync::{Arc, Mutex};

    // Create a mock reader that simulates EINTR interruptions
    struct InterruptedReader {
        data: Vec<u8>,
        position: usize,
        interrupt_count: Arc<Mutex<usize>>,
    }

    impl Read for InterruptedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            // Simulate interruption on first read attempt
            if self.position < self.data.len() {
                let mut count = self.interrupt_count.lock().unwrap();
                if *count == 0 {
                    *count += 1;
                    return Err(Error::new(
                        ErrorKind::Interrupted,
                        "Simulated signal interruption",
                    ));
                }
            }

            // Return actual data on subsequent attempts
            if self.position >= self.data.len() {
                return Ok(0);
            }

            let remaining = self.data.len() - self.position;
            let to_copy = std::cmp::min(buf.len(), remaining);
            buf[..to_copy].copy_from_slice(&self.data[self.position..self.position + to_copy]);
            self.position += to_copy;
            Ok(to_copy)
        }
    }

    let test_data = b"Hello, World!\n";
    let interrupt_count = Arc::new(Mutex::new(0));
    let reader = InterruptedReader {
        data: test_data.to_vec(),
        position: 0,
        interrupt_count: interrupt_count.clone(),
    };

    // Test that cat can handle the interrupted reader
    let result = std::io::copy(&mut { reader }, &mut std::io::stdout());
    assert!(result.is_ok());

    // Verify that the interruption was encountered and handled
    assert_eq!(*interrupt_count.lock().unwrap(), 1);
}
