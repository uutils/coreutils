// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) asdf algo algos

use crate::common::util::TestScenario;

const ALGOS: [&str; 11] = [
    "sysv", "bsd", "crc", "md5", "sha1", "sha224", "sha256", "sha384", "sha512", "blake2b", "sm3",
];

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_single_file() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("crc_single_file.expected");
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("crc_multiple_files.expected");
}

#[test]
fn test_stdin() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("crc_stdin.expected");
}

#[test]
fn test_empty_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");

    ucmd.arg("a")
        .succeeds()
        .no_stderr()
        .normalized_newlines_stdout_is("4294967295 0 a\n");
}

#[test]
fn test_arg_overrides_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    let input = "foobarfoobar"; // spell-checker:disable-line

    at.touch("a");

    ucmd.arg("a")
        .pipe_in(input.as_bytes())
        // the command might have exited before all bytes have been pipe in.
        // in that case, we don't care about the error (broken pipe)
        .ignore_stdin_write_error()
        .succeeds()
        .no_stderr()
        .normalized_newlines_stdout_is("4294967295 0 a\n");
}

#[test]
fn test_nonexisting_file() {
    let file_name = "asdf";

    new_ucmd!()
        .arg(file_name)
        .fails()
        .no_stdout()
        .stderr_contains(format!("cksum: {file_name}: No such file or directory"));
}

// Make sure crc is correct for files larger than 32 bytes
// but <128 bytes (1 fold pclmul) // spell-checker:disable-line
#[test]
fn test_crc_for_bigger_than_32_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("chars.txt").succeeds();

    let mut stdout_split = result.stdout_str().split(' ');

    let cksum: i64 = stdout_split.next().unwrap().parse().unwrap();
    let bytes_cnt: i64 = stdout_split.next().unwrap().parse().unwrap();

    assert_eq!(cksum, 586_047_089);
    assert_eq!(bytes_cnt, 16);
}

#[test]
fn test_stdin_larger_than_128_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("larger_than_2056_bytes.txt").succeeds();

    let mut stdout_split = result.stdout_str().split(' ');

    let cksum: i64 = stdout_split.next().unwrap().parse().unwrap();
    let bytes_cnt: i64 = stdout_split.next().unwrap().parse().unwrap();

    assert_eq!(cksum, 945_881_979);
    assert_eq!(bytes_cnt, 2058);
}

#[test]
fn test_algorithm_single_file() {
    for algo in ALGOS {
        for option in ["-a", "--algorithm"] {
            new_ucmd!()
                .arg(format!("{option}={algo}"))
                .arg("lorem_ipsum.txt")
                .succeeds()
                .stdout_is_fixture(format!("{algo}_single_file.expected"));
        }
    }
}

#[test]
fn test_algorithm_multiple_files() {
    for algo in ALGOS {
        for option in ["-a", "--algorithm"] {
            new_ucmd!()
                .arg(format!("{option}={algo}"))
                .arg("lorem_ipsum.txt")
                .arg("alice_in_wonderland.txt")
                .succeeds()
                .stdout_is_fixture(format!("{algo}_multiple_files.expected"));
        }
    }
}

#[test]
fn test_algorithm_stdin() {
    for algo in ALGOS {
        for option in ["-a", "--algorithm"] {
            new_ucmd!()
                .arg(format!("{option}={algo}"))
                .pipe_in_fixture("lorem_ipsum.txt")
                .succeeds()
                .stdout_is_fixture(format!("{algo}_stdin.expected"));
        }
    }
}

#[test]
fn test_untagged_single_file() {
    new_ucmd!()
        .arg("--untagged")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("untagged/crc_single_file.expected");
}

#[test]
fn test_untagged_multiple_files() {
    new_ucmd!()
        .arg("--untagged")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("untagged/crc_multiple_files.expected");
}

#[test]
fn test_untagged_stdin() {
    new_ucmd!()
        .arg("--untagged")
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("untagged/crc_stdin.expected");
}

#[test]
fn test_untagged_algorithm_single_file() {
    for algo in ALGOS {
        new_ucmd!()
            .arg("--untagged")
            .arg(format!("--algorithm={algo}"))
            .arg("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/{algo}_single_file.expected"));
    }
}

#[test]
fn test_untagged_algorithm_multiple_files() {
    for algo in ALGOS {
        new_ucmd!()
            .arg("--untagged")
            .arg(format!("--algorithm={algo}"))
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/{algo}_multiple_files.expected"));
    }
}

#[test]
fn test_untagged_algorithm_stdin() {
    for algo in ALGOS {
        new_ucmd!()
            .arg("--untagged")
            .arg(format!("--algorithm={algo}"))
            .pipe_in_fixture("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/{algo}_stdin.expected"));
    }
}

#[test]
fn test_length_with_wrong_algorithm() {
    new_ucmd!()
        .arg("--length=16")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: --length is only supported with --algorithm=blake2b")
        .code_is(1);
}

#[test]
fn test_length_not_supported() {
    new_ucmd!()
        .arg("--length=15")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_is_fixture("unsupported_length.expected")
        .code_is(1);
}

#[test]
fn test_length() {
    new_ucmd!()
        .arg("--length=16")
        .arg("--algorithm=blake2b")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("supported_length.expected");
}

#[test]
fn test_length_greater_than_512() {
    new_ucmd!()
        .arg("--length=1024")
        .arg("--algorithm=blake2b")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .fails()
        .no_stdout()
        .stderr_is_fixture("length_larger_than_512.expected");
}

#[test]
fn test_length_is_zero() {
    new_ucmd!()
        .arg("--length=0")
        .arg("--algorithm=blake2b")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("length_is_zero.expected");
}

#[test]
fn test_fail_on_folder() {
    let (at, mut ucmd) = at_and_ucmd!();

    let folder_name = "a_folder";
    at.mkdir(folder_name);

    ucmd.arg(folder_name)
        .fails()
        .no_stdout()
        .stderr_contains(format!("cksum: {folder_name}: Is a directory"));
}

#[test]
fn test_all_algorithms_fail_on_folder() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    let folder_name = "a_folder";
    at.mkdir(folder_name);

    for algo in ALGOS {
        scene
            .ucmd()
            .arg(format!("--algorithm={algo}"))
            .arg(folder_name)
            .fails()
            .no_stdout()
            .stderr_contains(format!("cksum: {folder_name}: Is a directory"));
    }
}
