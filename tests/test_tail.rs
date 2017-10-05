extern crate uu_tail;

use common::util::*;
use std::char::from_digit;
use self::uu_tail::parse_size;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;


static FOOBAR_TXT: &'static str = "foobar.txt";
static FOOBAR_2_TXT: &'static str = "foobar2.txt";
static FOOBAR_WITH_NULL_TXT: &'static str = "foobar_with_null.txt";

#[test]
fn test_stdin_default() {
    new_ucmd!().pipe_in_fixture(FOOBAR_TXT).run().stdout_is_fixture("foobar_stdin_default.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!().arg(FOOBAR_TXT).run().stdout_is_fixture("foobar_single_default.expected");
}

#[test]
fn test_n_greater_than_number_of_lines() {
    new_ucmd!().arg("-n").arg("99999999").arg(FOOBAR_TXT).run()
        .stdout_is_fixture(FOOBAR_TXT);
}

#[test]
fn test_null_default() {
    new_ucmd!().arg("-z").arg(FOOBAR_WITH_NULL_TXT).run().stdout_is_fixture("foobar_with_null_default.expected");
}

#[test]
fn test_follow() {
    let (at, mut ucmd) = at_and_ucmd!();

    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).run_no_wait();

    let expected = at.read("foobar_single_default.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    // We write in a temporary copy of foobar.txt
    let expected = "line1\nline2\n";
    at.append(FOOBAR_TXT, expected);

    assert_eq!(read_size(&mut child, expected.len()), expected);

    child.kill().unwrap();
}

#[test]
fn test_follow_multiple() {
    let (at, mut ucmd) = at_and_ucmd!();
    let mut child = ucmd.arg("-f").arg(FOOBAR_TXT).arg(FOOBAR_2_TXT).run_no_wait();

    let expected = at.read("foobar_follow_multiple.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);
    assert_eq!(read_size(&mut child, first_append.len()), first_append);

    let second_append = "doce\ntrece\n";
    let expected = at.read("foobar_follow_multiple_appended.expected");
    at.append(FOOBAR_TXT, second_append);
    assert_eq!(read_size(&mut child, expected.len()), expected);

    child.kill().unwrap();
}

#[test]
fn test_follow_stdin() {
    new_ucmd!().arg("-f").pipe_in_fixture(FOOBAR_TXT).run().stdout_is_fixture("follow_stdin.expected");
}

#[test]
fn test_follow_with_pid() {
    let (at, mut ucmd) = at_and_ucmd!();

    #[cfg(unix)]
    let dummy_cmd = "sh";
    #[cfg(windows)]
    let dummy_cmd = "cmd";

    let mut dummy = Command::new(dummy_cmd).stdout(Stdio::null()).spawn().unwrap();
    let pid = dummy.id();

    let mut child = ucmd.arg("-f").arg(format!("--pid={}", pid)).arg(FOOBAR_TXT).arg(FOOBAR_2_TXT).run_no_wait();

    let expected = at.read("foobar_follow_multiple.expected");
    assert_eq!(read_size(&mut child, expected.len()), expected);

    let first_append = "trois\n";
    at.append(FOOBAR_2_TXT, first_append);
    assert_eq!(read_size(&mut child, first_append.len()), first_append);

    let second_append = "doce\ntrece\n";
    let expected = at.read("foobar_follow_multiple_appended.expected");
    at.append(FOOBAR_TXT, second_append);
    assert_eq!(read_size(&mut child, expected.len()), expected);

    // kill the dummy process and give tail time to notice this
    dummy.kill().unwrap();
    let _ = dummy.wait();
    sleep(Duration::from_secs(1));

    let third_append = "should\nbe\nignored\n";
    at.append(FOOBAR_TXT, third_append);
    assert_eq!(read_size(&mut child, 1), "\u{0}");

    // On Unix, trying to kill a process that's already dead is fine; on Windows it's an error.
    #[cfg(unix)]
    child.kill().unwrap();
    #[cfg(windows)]
    assert_eq!(child.kill().is_err(), true);
}

#[test]
fn test_single_big_args() {
    const FILE: &'static str = "single_big_args.txt";
    const EXPECTED_FILE: &'static str = "single_big_args_expected.txt";
    const LINES: usize = 1_000_000;
    const N_ARG: usize = 100_000;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..LINES {
        write!(&mut big_input, "Line {}\n", i).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        write!(&mut big_expected, "Line {}\n", i).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    ucmd.arg(FILE).arg("-n").arg(format!("{}", N_ARG)).run().stdout_is(at.read(EXPECTED_FILE));
}

#[test]
fn test_bytes_single() {
    new_ucmd!().arg("-c").arg("10").arg(FOOBAR_TXT).run()
        .stdout_is_fixture("foobar_bytes_single.expected");
}

#[test]
fn test_bytes_stdin() {
    new_ucmd!().arg("-c").arg("13").pipe_in_fixture(FOOBAR_TXT).run()
            .stdout_is_fixture("foobar_bytes_stdin.expected");
}

#[test]
fn test_bytes_big() {
    const FILE: &'static str = "test_bytes_big.txt";
    const EXPECTED_FILE: &'static str = "test_bytes_big_expected.txt";
    const BYTES: usize = 1_000_000;
    const N_ARG: usize = 100_000;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..BYTES {
        let digit = from_digit((i % 10) as u32, 10).unwrap();
        write!(&mut big_input, "{}", digit).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (BYTES - N_ARG)..BYTES {
        let digit = from_digit((i % 10) as u32, 10).unwrap();
        write!(&mut big_expected, "{}", digit).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    let result = ucmd.arg(FILE).arg("-c").arg(format!("{}", N_ARG)).run().stdout;
    let expected = at.read(EXPECTED_FILE);

    assert_eq!(result.len(), expected.len());
    for (actual_char, expected_char) in result.chars().zip(expected.chars()) {
        assert_eq!(actual_char, expected_char);
    }
}

#[test]
fn test_parse_size() {
    // No suffix.
    assert_eq!(Ok(1234), parse_size("1234"));

    // kB is 1000
    assert_eq!(Ok(9 * 1000), parse_size("9kB"));

    // K is 1024
    assert_eq!(Ok(2 * 1024), parse_size("2K"));

    let suffixes = [
        ('M', 2u32),
        ('G', 3u32),
        ('T', 4u32),
        ('P', 5u32),
        ('E', 6u32),
    ];

    for &(c, exp) in &suffixes {
        let s = format!("2{}B", c);
        assert_eq!(Ok(2 * (1000 as u64).pow(exp)), parse_size(&s));

        let s = format!("2{}", c);
        assert_eq!(Ok(2 * (1024 as u64).pow(exp)), parse_size(&s));
    }

    // Sizes that are too big.
    assert!(parse_size("1Z").is_err());
    assert!(parse_size("1Y").is_err());

    // Bad number
    assert!(parse_size("328hdsf3290").is_err());
}

#[test]
fn test_lines_with_size_suffix() {
    const FILE: &'static str = "test_lines_with_size_suffix.txt";
    const EXPECTED_FILE: &'static str = "test_lines_with_size_suffix_expected.txt";
    const LINES: usize = 3_000;
    const N_ARG: usize = 2 * 1024;

    let (at, mut ucmd) = at_and_ucmd!();

    let mut big_input = at.make_file(FILE);
    for i in 0..LINES {
        writeln!(&mut big_input, "Line {}", i).expect("Could not write to FILE");
    }
    big_input.flush().expect("Could not flush FILE");

    let mut big_expected = at.make_file(EXPECTED_FILE);
    for i in (LINES - N_ARG)..LINES {
        writeln!(&mut big_expected, "Line {}", i).expect("Could not write to EXPECTED_FILE");
    }
    big_expected.flush().expect("Could not flush EXPECTED_FILE");

    ucmd.arg(FILE).arg("-n").arg("2K").run().stdout_is_fixture(EXPECTED_FILE);
}

#[test]
fn test_multiple_input_files() {
    new_ucmd!().arg(FOOBAR_TXT).arg(FOOBAR_2_TXT).run().stdout_is_fixture("foobar_follow_multiple.expected");
}

#[test]
fn test_multiple_input_files_with_suppressed_headers() {
    new_ucmd!().arg(FOOBAR_TXT).arg(FOOBAR_2_TXT).arg("-q").run().stdout_is_fixture("foobar_multiple_quiet.expected");
}

#[test]
fn test_multiple_input_quiet_flag_overrides_verbose_flag_for_suppressing_headers() {
    // TODO: actually the later one should win, i.e. -qv should lead to headers being printed, -vq to them being suppressed
    new_ucmd!().arg(FOOBAR_TXT).arg(FOOBAR_2_TXT).arg("-q").arg("-v").run().stdout_is_fixture("foobar_multiple_quiet.expected");
}
