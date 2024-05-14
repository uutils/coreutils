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

#[test]
fn test_nonexisting_file_out() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(
        "f",
        "MD5 (nonexistent) = e5773576fc75ff0f8eba14f61587ae28\n",
    );

    ucmd.arg("-c")
        .arg("f")
        .fails()
        .stdout_contains("nonexistent: FAILED open or read")
        .stderr_contains("cksum: nonexistent: No such file or directory");
}

#[test]
fn test_one_nonexisting_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("abc.txt");
    at.touch("xyz.txt");

    ucmd.arg("abc.txt")
        .arg("asdf.txt")
        .arg("xyz.txt")
        .fails()
        .stdout_contains_line("4294967295 0 xyz.txt")
        .stderr_contains("asdf.txt: No such file or directory")
        .stdout_contains_line("4294967295 0 abc.txt");
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
fn test_repeated_flags() {
    new_ucmd!()
        .arg("-a")
        .arg("sha1")
        .arg("--algo=sha256")
        .arg("-a=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("md5_single_file.expected");
}

#[test]
fn test_tag_after_untagged() {
    new_ucmd!()
        .arg("--untagged")
        .arg("--tag")
        .arg("-a=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("md5_single_file.expected");
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
fn test_tag_short() {
    new_ucmd!()
        .arg("-t")
        .arg("--untagged")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa  lorem_ipsum.txt\n");
}

#[test]
fn test_untagged_algorithm_after_tag() {
    new_ucmd!()
        .arg("--tag")
        .arg("--untagged")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("untagged/md5_single_file.expected");
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
fn test_check_algo() {
    new_ucmd!()
        .arg("-a=bsd")
        .arg("--check")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: --check is not supported with --algorithm={bsd,sysv,crc}")
        .code_is(1);
    new_ucmd!()
        .arg("-a=sysv")
        .arg("--check")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: --check is not supported with --algorithm={bsd,sysv,crc}")
        .code_is(1);
    new_ucmd!()
        .arg("-a=crc")
        .arg("--check")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: --check is not supported with --algorithm={bsd,sysv,crc}")
        .code_is(1);
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

    new_ucmd!()
        .arg("--length=16")
        .arg("--algorithm=md5")
        .arg("-c")
        .arg("foo.sums")
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
        .stderr_contains("--length is only supported with --algorithm=blake2b")
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
        .stdout_contains(
            "BLAKE2b-16 (lorem_ipsum.txt) = 7e2f\nBLAKE2b-16 (alice_in_wonderland.txt) = a546",
        );
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
fn test_length_repeated() {
    new_ucmd!()
        .arg("--length=10")
        .arg("--length=123456")
        .arg("--length=0")
        .arg("--algorithm=blake2b")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("length_is_zero.expected");
}

#[test]
fn test_raw_single_file() {
    for algo in ALGOS {
        new_ucmd!()
            .arg("--raw")
            .arg("lorem_ipsum.txt")
            .arg(format!("--algorithm={algo}"))
            .succeeds()
            .no_stderr()
            .stdout_is_fixture_bytes(format!("raw/{algo}_single_file.expected"));
    }
}
#[test]
fn test_raw_multiple_files() {
    new_ucmd!()
        .arg("--raw")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: the --raw option is not supported with multiple files")
        .code_is(1);
}

#[test]
fn test_base64_raw_conflicts() {
    new_ucmd!()
        .arg("--base64")
        .arg("--raw")
        .arg("lorem_ipsum.txt")
        .fails()
        .no_stdout()
        .stderr_contains("--base64")
        .stderr_contains("cannot be used with")
        .stderr_contains("--raw");
}

#[test]
fn test_base64_single_file() {
    for algo in ALGOS {
        new_ucmd!()
            .arg("--base64")
            .arg("lorem_ipsum.txt")
            .arg(format!("--algorithm={algo}"))
            .succeeds()
            .no_stderr()
            .stdout_is_fixture_bytes(format!("base64/{algo}_single_file.expected"));
    }
}
#[test]
fn test_base64_multiple_files() {
    new_ucmd!()
        .arg("--base64")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .no_stderr()
        .stdout_is_fixture_bytes("base64/md5_multiple_files.expected");
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

#[cfg(unix)]
#[test]
fn test_check_error_incorrect_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("checksum", "e5773576fc75ff0f8eba14f61587ae28  README.md");

    scene
        .ucmd()
        .arg("-c")
        .arg("checksum")
        .fails()
        .stderr_contains("no properly formatted checksum lines found");
}

#[cfg(unix)]
#[test]
fn test_dev_null() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("--tag")
        .arg("--untagged")
        .arg("--algorithm=md5")
        .arg("/dev/null")
        .succeeds()
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e ");
}

#[cfg(unix)]
#[test]
fn test_blake2b_512() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("-a")
        .arg("blake2b")
        .arg("-l512")
        .arg("f")
        .succeeds()
        .stdout_contains("BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce");

    // test also the read
    at.write("checksum", "BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce");
    scene
        .ucmd()
        .arg("--check")
        .arg("checksum")
        .succeeds()
        .stdout_contains("f: OK");
}

#[test]
fn test_reset_binary() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--binary") // should disappear because of the following option
        .arg("--tag")
        .arg("--untagged")
        .arg("--algorithm=md5")
        .arg(at.subdir.join("f"))
        .succeeds()
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e  ");
}

#[ignore = "issue #6375"]
#[test]
fn test_reset_binary_but_set() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--binary")
        .arg("--tag")
        .arg("--untagged")
        .arg("--binary")
        .arg("--algorithm=md5")
        .arg(at.subdir.join("f"))
        .succeeds()
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e *"); // currently, asterisk=false. It should be true
}

#[test]
fn test_text_tag() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--text") // should disappear because of the following option
        .arg("--tag")
        .arg(at.subdir.join("f"))
        .succeeds()
        .stdout_contains("4294967295 0 ");
}

#[test]
fn test_binary_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--untagged")
        .arg("-b")
        .arg("--algorithm=md5")
        .arg(at.subdir.join("f"))
        .succeeds()
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e *");

    scene
        .ucmd()
        .arg("--tag")
        .arg("--untagged")
        .arg("--binary")
        .arg("--algorithm=md5")
        .arg(at.subdir.join("f"))
        .succeeds()
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e *");

    scene
        .ucmd()
        .arg("--tag")
        .arg("--untagged")
        .arg("--binary")
        .arg("--algorithm=md5")
        .arg("raw/blake2b_single_file.expected")
        .succeeds()
        .stdout_contains("7e297c07ed8e053600092f91bdd1dad7 *");

    new_ucmd!()
        .arg("--tag")
        .arg("--untagged")
        .arg("--binary")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa *lorem_ipsum.txt\n");

    new_ucmd!()
        .arg("--untagged")
        .arg("--binary")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa *lorem_ipsum.txt\n");

    new_ucmd!()
        .arg("--binary")
        .arg("--untagged")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa *lorem_ipsum.txt\n");

    new_ucmd!()
        .arg("-a")
        .arg("md5")
        .arg("--binary")
        .arg("--untagged")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa *lorem_ipsum.txt\n");

    new_ucmd!()
        .arg("-a")
        .arg("md5")
        .arg("--binary")
        .arg("--tag")
        .arg("--untagged")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is("cd724690f7dc61775dfac400a71f2caa  lorem_ipsum.txt\n");
}

#[test]
fn test_folder_and_file() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    let folder_name = "a_folder";
    at.mkdir(folder_name);

    scene
        .ucmd()
        .arg(folder_name)
        .arg("lorem_ipsum.txt")
        .fails()
        .stderr_contains(format!("cksum: {folder_name}: Is a directory"))
        .stdout_is_fixture("crc_single_file.expected");
}

#[test]
fn test_conflicting_options() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--binary")
        .arg("--check")
        .arg("f")
        .fails()
        .no_stdout()
        .stderr_contains(
            "cksum: the --binary and --text options are meaningless when verifying checksums",
        )
        .code_is(1);
}

#[test]
fn test_check_algo_err() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("-a")
        .arg("sm3")
        .arg("--check")
        .arg("f")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: f: no properly formatted checksum lines found")
        .code_is(1);
}

#[test]
fn test_check_pipe() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;

    at.touch("f");

    scene
        .ucmd()
        .arg("--check")
        .arg("-")
        .pipe_in("f")
        .fails()
        .no_stdout()
        .stderr_contains("cksum: 'standard input': no properly formatted checksum lines found")
        .code_is(1);
}

#[test]
fn test_cksum_check_empty_line() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("CHECKSUM", "\
    SHA384 (f) = 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b\n\
    BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n\
    BLAKE2b-384 (f) = b32811423377f52d7862286ee1a72ee540524380fda1724a6f25d7978c6fd3244a6caf0498812673c5e05ef583825100\n\
    SM3 (f) = 1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b\n\n");
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_does_not_contain("line is improperly formatted");
}

#[test]
fn test_cksum_check_space() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("CHECKSUM", "\
    SHA384 (f) = 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b\n\
    BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n\
    BLAKE2b-384 (f) = b32811423377f52d7862286ee1a72ee540524380fda1724a6f25d7978c6fd3244a6caf0498812673c5e05ef583825100\n\
    SM3 (f) = 1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b\n  \n");
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_contains("line is improperly formatted");
}

#[test]
fn test_cksum_check() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("CHECKSUM", "\
    SHA384 (f) = 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b\n\
    BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n\
    BLAKE2b-384 (f) = b32811423377f52d7862286ee1a72ee540524380fda1724a6f25d7978c6fd3244a6caf0498812673c5e05ef583825100\n\
    SM3 (f) = 1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b\n");
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_does_not_contain("line is improperly formatted");
    scene
        .ucmd()
        .arg("--check")
        .arg("--strict")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_does_not_contain("line is improperly formatted");
    // inject invalid content
    at.append("CHECKSUM", "incorrect data");
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_contains("line is improperly formatted");
    scene
        .ucmd()
        .arg("--check")
        .arg("--strict")
        .arg("CHECKSUM")
        .fails()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n")
        .stderr_contains("line is improperly formatted");
}

#[test]
fn test_cksum_check_case() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write(
        "CHECKSUM",
        "Sha1 (f) = da39a3ee5e6b4b0d3255bfef95601890afd80709\n",
    );
    scene.ucmd().arg("--check").arg("CHECKSUM").fails();
}

#[test]
fn test_cksum_check_invalid() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let commands = [vec!["-a", "sha384"]];
    at.touch("f");
    at.touch("CHECKSUM");
    for command in &commands {
        let result = scene.ucmd().args(command).arg("f").succeeds();
        at.append("CHECKSUM", result.stdout_str());
    }
    // inject invalid content
    at.append("CHECKSUM", "again incorrect data\naze\n");
    scene
        .ucmd()
        .arg("--check")
        .arg("--strict")
        .arg("CHECKSUM")
        .fails()
        .stdout_contains("f: OK\n")
        .stderr_contains("2 lines");

    // without strict, it passes
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\n")
        .stderr_contains("2 lines");
}

#[test]
fn test_cksum_check_failed() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let commands = [vec!["-a", "sha384"]];
    at.touch("f");
    at.touch("CHECKSUM");
    for command in &commands {
        let result = scene.ucmd().args(command).arg("f").succeeds();
        at.append("CHECKSUM", result.stdout_str());
    }
    // inject invalid content
    at.append("CHECKSUM", "again incorrect data\naze\nSM3 (input) = 7cfb120d4fabea2a904948538a438fdb57c725157cb40b5aee8d937b8351477e\n");

    let result = scene
        .ucmd()
        .arg("--check")
        .arg("--strict")
        .arg("CHECKSUM")
        .fails();

    assert!(result.stderr_str().contains("input: No such file or directory"));
    assert!(result
        .stderr_str()
        .contains("2 lines are improperly formatted\n"));
    assert!(result
        .stderr_str()
        .contains("1 listed file could not be read\n"));
    assert!(result.stdout_str().contains("f: OK\n"));

    // without strict
    let result = scene.ucmd().arg("--check").arg("CHECKSUM").fails();

    assert!(result.stderr_str().contains("input: No such file or directory"));
    assert!(result
        .stderr_str()
        .contains("2 lines are improperly formatted\n"));
    assert!(result
        .stderr_str()
        .contains("1 listed file could not be read\n"));
    assert!(result.stdout_str().contains("f: OK\n"));

    // tests with two files
    at.touch("CHECKSUM2");
    at.write("f2", "42");
    for command in &commands {
        let result = scene.ucmd().args(command).arg("f2").succeeds();
        at.append("CHECKSUM2", result.stdout_str());
    }
    // inject invalid content
    at.append("CHECKSUM2", "again incorrect data\naze\nSM3 (input2) = 7cfb120d4fabea2a904948538a438fdb57c725157cb40b5aee8d937b8351477e\n");
    at.append("CHECKSUM2", "again incorrect data\naze\nSM3 (input2) = 7cfb120d4fabea2a904948538a438fdb57c725157cb40b5aee8d937b8351477e\n");

    let result = scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .arg("CHECKSUM2")
        .fails();
    println!("result.stderr_str() {}", result.stderr_str());
    println!("result.stdout_str() {}", result.stdout_str());
    assert!(result.stderr_str().contains("input2: No such file or directory"));
    assert!(result
        .stderr_str()
        .contains("4 lines are improperly formatted\n"));
    assert!(result
        .stderr_str()
        .contains("2 listed files could not be read\n"));
    assert!(result.stdout_str().contains("f: OK\n"));
    assert!(result.stdout_str().contains("2: OK\n"));
}

#[test]
fn test_check_md5_format() {
    let scene = TestScenario::new(util_name!());

    let at = &scene.fixtures;
    at.touch("empty");
    at.write("f", "d41d8cd98f00b204e9800998ecf8427e *empty\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("md5")
        .arg("--check")
        .arg("f")
        .succeeds()
        .stdout_contains("empty: OK");

    // with a second file
    at.write("not-empty", "42");
    at.write("f2", "a1d0c6e83f027327d8461063f4ac58a6 *not-empty\n");

    scene
        .ucmd()
        .arg("-a")
        .arg("md5")
        .arg("--check")
        .arg("f")
        .arg("f2")
        .succeeds()
        .stdout_contains("empty: OK")
        .stdout_contains("not-empty: OK");
}

// Manage the mixed behavior
// cksum --check -a sm3 CHECKSUMS
// when CHECKSUM contains among other lines:
// SHA384 (input) = f392fd0ae43879ced890c665a1d47179116b5eddf6fb5b49f4982746418afdcbd54ba5eedcd422af3592f57f666da285
#[test]
fn test_cksum_mixed() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let commands = [
        vec!["-a", "sha384"],
        vec!["-a", "blake2b"],
        vec!["-a", "blake2b", "-l", "384"],
        vec!["-a", "sm3"],
    ];
    at.touch("f");
    at.touch("CHECKSUM");
    for command in &commands {
        let result = scene.ucmd().args(command).arg("f").succeeds();
        at.append("CHECKSUM", result.stdout_str());
    }
    scene
        .ucmd()
        .arg("--check")
        .arg("-a")
        .arg("sm3")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK")
        .stderr_contains("3 lines are improperly formatted");
}

#[test]
fn test_cksum_garbage() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Incorrect data at the start
    at.write(
        "check-file",
        "garbage MD5 (README.md) = e5773576fc75ff0f8eba14f61587ae28",
    );
    scene
        .ucmd()
        .arg("--check")
        .arg("check-file")
        .fails()
        .stderr_contains("check-file: no properly formatted checksum lines found");

    // Incorrect data at the end
    at.write(
        "check-file",
        "MD5 (README.md) = e5773576fc75ff0f8eba14f61587ae28 garbage",
    );
    scene
        .ucmd()
        .arg("--check")
        .arg("check-file")
        .fails()
        .stderr_contains("check-file: no properly formatted checksum lines found");
}
