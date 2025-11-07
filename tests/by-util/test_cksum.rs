// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) asdf algo algos asha mgmt xffname hexa GFYEQ HYQK Yqxb dont

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util::log_info;
use uutests::util_name;

const ALGOS: [&str; 12] = [
    "sysv", "bsd", "crc", "crc32b", "md5", "sha1", "sha224", "sha256", "sha384", "sha512",
    "blake2b", "sm3",
];
const SHA_LENGTHS: [u32; 4] = [224, 256, 384, 512];

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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
        .fails_with_code(1)
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
    let result = new_ucmd!().arg("chars.txt").succeeds();

    let mut stdout_split = result.stdout_str().split(' ');

    let cksum: i64 = stdout_split.next().unwrap().parse().unwrap();
    let bytes_cnt: i64 = stdout_split.next().unwrap().parse().unwrap();

    assert_eq!(cksum, 586_047_089);
    assert_eq!(bytes_cnt, 16);
}

#[test]
fn test_stdin_larger_than_128_bytes() {
    let result = new_ucmd!().arg("larger_than_2056_bytes.txt").succeeds();

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
fn test_sha_length_invalid() {
    for algo in ["sha2", "sha3"] {
        for l in ["0", "00", "13", "56", "99999999999999999999999999"] {
            new_ucmd!()
                .arg("--algorithm")
                .arg(algo)
                .arg("--length")
                .arg(l)
                .arg("/dev/null")
                .fails_with_code(1)
                .no_stdout()
                .stderr_contains(format!("invalid length: '{l}'"))
                .stderr_contains(format!(
                    "digest length for '{}' must be 224, 256, 384, or 512",
                    algo.to_ascii_uppercase()
                ));

            // Also fails with --check
            new_ucmd!()
                .arg("--algorithm")
                .arg(algo)
                .arg("--length")
                .arg(l)
                .arg("/dev/null")
                .arg("--check")
                .fails_with_code(1)
                .no_stdout()
                .stderr_contains(format!("invalid length: '{l}'"))
                .stderr_contains(format!(
                    "digest length for '{}' must be 224, 256, 384, or 512",
                    algo.to_ascii_uppercase()
                ));
        }

        // Different error for NaNs
        for l in ["512x", "x512", "512x512"] {
            new_ucmd!()
                .arg("--algorithm")
                .arg(algo)
                .arg("--length")
                .arg(l)
                .arg("/dev/null")
                .fails_with_code(1)
                .no_stdout()
                .stderr_contains(format!("invalid length: '{l}'"));

            // Also fails with --check
            new_ucmd!()
                .arg("--algorithm")
                .arg(algo)
                .arg("--length")
                .arg(l)
                .arg("/dev/null")
                .arg("--check")
                .fails_with_code(1)
                .no_stdout()
                .stderr_contains(format!("invalid length: '{l}'"));
        }
    }
}

#[test]
fn test_sha_missing_length() {
    for algo in ["sha2", "sha3"] {
        new_ucmd!()
            .arg("--algorithm")
            .arg(algo)
            .arg("lorem_ipsum.txt")
            .fails_with_code(1)
            .no_stdout()
            .stderr_contains(format!(
                "--algorithm={algo} requires specifying --length 224, 256, 384, or 512"
            ));
    }
}

#[test]
fn test_sha2_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha{l}_single_file.expected"));
    }
}

#[test]
fn test_sha2_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha{l}_multiple_files.expected"));
    }
}

#[test]
fn test_sha2_stdin() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .pipe_in_fixture("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha{l}_stdin.expected"));
    }
}

#[test]
fn test_untagged_sha2_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha{l}_single_file.expected"));
    }
}

#[test]
fn test_untagged_sha2_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha{l}_multiple_files.expected"));
    }
}

#[test]
fn test_untagged_sha2_stdin() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha2")
            .arg(format!("--length={l}"))
            .pipe_in_fixture("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha{l}_stdin.expected"));
    }
}

#[test]
fn test_check_tagged_sha2_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg(format!("sha{l}_single_file.expected"))
            .succeeds()
            .stdout_is("lorem_ipsum.txt: OK\n");
    }
}

#[test]
fn test_check_tagged_sha2_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg(format!("sha{l}_multiple_files.expected"))
            .succeeds()
            .stdout_contains("lorem_ipsum.txt: OK\n")
            .stdout_contains("alice_in_wonderland.txt: OK\n");
    }
}

// When checking sha2 in untagged mode, the length is automatically deduced
// from the length of the digest.
#[test]
fn test_check_untagged_sha2_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg("--algorithm=sha2")
            .arg(format!("untagged/sha{l}_single_file.expected"))
            .succeeds()
            .stdout_is("lorem_ipsum.txt: OK\n");
    }
}

#[test]
fn test_check_untagged_sha2_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg("--algorithm=sha2")
            .arg(format!("untagged/sha{l}_multiple_files.expected"))
            .succeeds()
            .stdout_contains("lorem_ipsum.txt: OK\n")
            .stdout_contains("alice_in_wonderland.txt: OK\n");
    }
}

#[test]
fn test_check_sha2_tagged_variant() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");

    // SHA2-xxx is an alias to SHAxxx we don't output but we still recognize.
    let checksum_lines = [
        (
            "SHA224",
            "SHA2-224",
            "(f) = d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f",
        ),
        (
            "SHA256",
            "SHA2-256",
            "(f) = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "SHA384",
            "SHA2-384",
            "(f) = 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b",
        ),
        (
            "SHA512",
            "SHA2-512",
            "(f) = cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
        ),
    ];

    for (basic, variant, digest) in checksum_lines {
        let stdin = format!("{basic} {digest}");
        log_info("stdin is: ", &stdin);
        scene
            .ucmd()
            .arg("--check")
            .arg("--algorithm=sha2")
            .pipe_in(stdin)
            .succeeds()
            .stdout_is("f: OK\n");

        // Check that the variant works the same
        let stdin = format!("{variant} {digest}");
        log_info("stdin is: ", &stdin);
        scene
            .ucmd()
            .arg("--check")
            .arg("--algorithm=sha2")
            .pipe_in(stdin)
            .succeeds()
            .stdout_is("f: OK\n");
    }
}

#[test]
fn test_sha3_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha3_{l}_single_file.expected"));
    }
}

#[test]
fn test_sha3_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha3_{l}_multiple_files.expected"));
    }
}

#[test]
fn test_sha3_stdin() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .pipe_in_fixture("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("sha3_{l}_stdin.expected"));
    }
}

#[test]
fn test_untagged_sha3_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha3_{l}_single_file.expected"));
    }
}

#[test]
fn test_untagged_sha3_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha3_{l}_multiple_files.expected"));
    }
}

#[test]
fn test_untagged_sha3_stdin() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--untagged")
            .arg("--algorithm=sha3")
            .arg(format!("--length={l}"))
            .pipe_in_fixture("lorem_ipsum.txt")
            .succeeds()
            .stdout_is_fixture(format!("untagged/sha3_{l}_stdin.expected"));
    }
}

#[test]
fn test_check_tagged_sha3_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg(format!("sha3_{l}_single_file.expected"))
            .succeeds()
            .stdout_is("lorem_ipsum.txt: OK\n");
    }
}

#[test]
fn test_check_tagged_sha3_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg(format!("sha3_{l}_multiple_files.expected"))
            .succeeds()
            .stdout_contains("lorem_ipsum.txt: OK\n")
            .stdout_contains("alice_in_wonderland.txt: OK\n");
    }
}

// When checking sha3 in untagged mode, the length is automatically deduced
// from the length of the digest.
#[test]
fn test_check_untagged_sha3_single_file() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg("--algorithm=sha3")
            .arg(format!("untagged/sha3_{l}_single_file.expected"))
            .succeeds()
            .stdout_is("lorem_ipsum.txt: OK\n");
    }
}

#[test]
fn test_check_untagged_sha3_multiple_files() {
    for l in SHA_LENGTHS {
        new_ucmd!()
            .arg("--check")
            .arg("--algorithm=sha3")
            .arg(format!("untagged/sha3_{l}_multiple_files.expected"))
            .succeeds()
            .stdout_contains("lorem_ipsum.txt: OK\n")
            .stdout_contains("alice_in_wonderland.txt: OK\n");
    }
}

#[test]
fn test_check_algo() {
    for algo in ["bsd", "sysv", "crc", "crc32b"] {
        new_ucmd!()
            .arg("-a")
            .arg(algo)
            .arg("--check")
            .arg("lorem_ipsum.txt")
            .fails()
            .no_stdout()
            .stderr_contains(
                "cksum: --check is not supported with --algorithm={bsd,sysv,crc,crc32b}",
            );
    }
}

#[test]
fn test_length_with_wrong_algorithm() {
    new_ucmd!()
        .arg("--length=16")
        .arg("--algorithm=md5")
        .arg("lorem_ipsum.txt")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: --length is only supported with --algorithm blake2b, sha2, or sha3",
        );

    new_ucmd!()
        .arg("--length=16")
        .arg("--algorithm=md5")
        .arg("-c")
        .arg("foo.sums")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: --length is only supported with --algorithm blake2b, sha2, or sha3",
        );
}

/// Giving --length to a wrong algorithm doesn't fail if the length is zero
#[test]
fn test_length_is_zero_with_wrong_algorithm() {
    for algo in ["md5", "crc", "sha1", "sha224", "sha256", "sha384", "sha512"] {
        new_ucmd!()
            .arg("--length=0")
            .args(&["-a", algo])
            .arg("lorem_ipsum.txt")
            .succeeds()
            .no_stderr()
            .stdout_is_fixture(format!("{algo}_single_file.expected"));
    }
}

#[test]
fn test_length_not_supported() {
    new_ucmd!()
        .arg("--length=15")
        .arg("lorem_ipsum.txt")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: --length is only supported with --algorithm blake2b, sha2, or sha3",
        );

    new_ucmd!()
        .arg("-l")
        .arg("158")
        .arg("-c")
        .arg("-a")
        .arg("crc")
        .arg("/tmp/xxx")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: --length is only supported with --algorithm blake2b, sha2, or sha3",
        );
}

#[test]
fn test_blake2b_length() {
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
fn test_blake2b_length_greater_than_512() {
    new_ucmd!()
        .arg("--length=1024")
        .arg("--algorithm=blake2b")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .fails_with_code(1)
        .no_stdout()
        .stderr_is_fixture("length_larger_than_512.expected");
}

#[test]
fn test_blake2b_length_is_zero() {
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
fn test_blake2b_length_repeated() {
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
fn test_blake2b_length_invalid() {
    for len in [
        "1", "01", // Odd
        "",
    ] {
        new_ucmd!()
            .arg("--length")
            .arg(len)
            .arg("--algorithm=blake2b")
            .arg("lorem_ipsum.txt")
            .arg("alice_in_wonderland.txt")
            .fails_with_code(1)
            .stderr_contains(format!("invalid length: '{len}'"));
    }
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
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains("cksum: the --raw option is not supported with multiple files");
}

#[test]
fn test_base64_raw_conflicts() {
    new_ucmd!()
        .arg("--base64")
        .arg("--raw")
        .arg("lorem_ipsum.txt")
        .fails_with_code(1)
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

    scene
        .ucmd()
        .arg("--status")
        .arg("--check")
        .arg("checksum")
        .succeeds()
        .no_output();
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
        .stdout_contains("d41d8cd98f00b204e9800998ecf8427e *");
}

/// Test legacy behaviors with --tag, --untagged, --binary and --text
mod output_format {
    use super::*;

    #[test]
    fn test_text_tag() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");

        ucmd.arg("--text") // should disappear because of the following option
            .arg("--tag")
            .args(&["-a", "md5"])
            .arg(at.subdir.join("f"))
            .succeeds()
            // Tagged output is used
            .stdout_contains("f) = d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_text_no_untagged() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");

        // --text without --untagged fails
        ucmd.arg("--text")
            .args(&["-a", "md5"])
            .arg(at.subdir.join("f"))
            .fails_with_code(1)
            .stderr_contains("--text mode is only supported with --untagged");
    }

    #[test]
    fn test_text_binary() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");

        // --binary overwrites --text, thus no error is raised
        ucmd.arg("--text")
            .arg("--binary")
            .args(&["-a", "md5"])
            .arg(at.subdir.join("f"))
            .succeeds()
            // No --untagged, tagged output is used
            .stdout_contains("f) = d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_text_binary_untagged() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("f");

        // --binary overwrites --text
        ucmd.arg("--text")
            .arg("--binary")
            .arg("--untagged")
            .args(&["-a", "md5"])
            .arg(at.subdir.join("f"))
            .succeeds()
            // Untagged output is used
            .stdout_contains("d41d8cd98f00b204e9800998ecf8427e *");
    }
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
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: the --binary and --text options are meaningless when verifying checksums",
        );

    scene
        .ucmd()
        .arg("--tag")
        .arg("-c")
        .arg("-a")
        .arg("md5")
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains(
            "cksum: the --binary and --text options are meaningless when verifying checksums",
        );
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
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains("cksum: f: no properly formatted checksum lines found");
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
        .fails_with_code(1)
        .no_stdout()
        .stderr_contains("cksum: 'standard input': no properly formatted checksum lines found");
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
fn test_cksum_check_leading_info() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");
    at.write("CHECKSUM", "\
     \\SHA384 (f) = 38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b\n\
     \\BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n\
     \\BLAKE2b-384 (f) = b32811423377f52d7862286ee1a72ee540524380fda1724a6f25d7978c6fd3244a6caf0498812673c5e05ef583825100\n\
     \\SM3 (f) = 1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b\n");
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .succeeds()
        .stdout_contains("f: OK\nf: OK\nf: OK\nf: OK\n");
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

    assert!(
        result
            .stderr_str()
            .contains("input: No such file or directory")
    );
    assert!(
        result
            .stderr_str()
            .contains("2 lines are improperly formatted\n")
    );
    assert!(
        result
            .stderr_str()
            .contains("1 listed file could not be read\n")
    );
    assert!(result.stdout_str().contains("f: OK\n"));

    // without strict
    let result = scene.ucmd().arg("--check").arg("CHECKSUM").fails();

    assert!(
        result
            .stderr_str()
            .contains("input: No such file or directory")
    );
    assert!(
        result
            .stderr_str()
            .contains("2 lines are improperly formatted\n")
    );
    assert!(
        result
            .stderr_str()
            .contains("1 listed file could not be read\n")
    );
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
    assert!(
        result
            .stderr_str()
            .contains("input2: No such file or directory")
    );
    assert!(
        result
            .stderr_str()
            .contains("4 lines are improperly formatted\n")
    );
    assert!(
        result
            .stderr_str()
            .contains("2 listed files could not be read\n")
    );
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
    println!("Content of CHECKSUM:\n{}", at.read("CHECKSUM"));
    let result = scene
        .ucmd()
        .arg("--check")
        .arg("-a")
        .arg("sm3")
        .arg("CHECKSUM")
        .succeeds();

    println!("result.stderr_str() {}", result.stderr_str());
    println!("result.stdout_str() {}", result.stdout_str());
    assert!(result.stdout_str().contains("f: OK"));
    assert!(
        result
            .stderr_str()
            .contains("3 lines are improperly formatted")
    );
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

#[test]
fn test_md5_bits() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(
        "f",
        "MD5-65536 (README.md) = e5773576fc75ff0f8eba14f61587ae28\n",
    );

    ucmd.arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
}

#[test]
fn test_blake2b_bits() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(
        "f",
        "BLAKE2b-257 (README.md) = f9a984b70cf9a7549920864860fd1131c9fb6c0552def0b6dcce1d87b4ec4c5d\n"
    );

    ucmd.arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
}

#[test]
fn test_bsd_case() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.write("f", "bSD (README.md) = 0000\n");

    scene
        .ucmd()
        .arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
    at.write("f", "BsD (README.md) = 0000\n");

    scene
        .ucmd()
        .arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
}

#[test]
fn test_blake2d_tested_with_sha1() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(
        "f",
        "BLAKE2b (f) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n"
    );

    ucmd.arg("-a")
        .arg("sha1")
        .arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
}

#[test]
fn test_unknown_sha() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("f", "SHA4 (README.md) = 00000000\n");

    ucmd.arg("-c")
        .arg("f")
        .fails()
        .stderr_contains("f: no properly formatted checksum lines found");
}

#[test]
fn test_check_directory_error() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("d");
    at.write(
        "f",
        "BLAKE2b (d) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n"
    );
    #[cfg(not(windows))]
    let err_msg = "cksum: d: Is a directory\n";
    #[cfg(windows)]
    let err_msg = "cksum: d: Permission denied\n";
    ucmd.arg("--check")
        .arg(at.subdir.join("f"))
        .fails()
        .stderr_contains(err_msg);
}

#[test]
fn test_check_base64_hashes() {
    let hashes = "MD5 (empty) = 1B2M2Y8AsgTpgAmY7PhCfg==\nSHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\nBLAKE2b (empty) = eGoC90IBWQPGxv2FJVLScpEvR0DhWEdhiobiF/cfVBnSXhAxr+5YUxOJZESTTrBLkDpoWxRIt1XVb3Aa/pvizg==\n";

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");
    at.write("check", hashes);

    scene
        .ucmd()
        .arg("--check")
        .arg(at.subdir.join("check"))
        .succeeds()
        .stdout_is("empty: OK\nempty: OK\nempty: OK\n");
}

#[test]
fn test_several_files_error_mgmt() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // don't exist
    scene
        .ucmd()
        .arg("--check")
        .arg("empty")
        .arg("incorrect")
        .fails()
        .stderr_contains("empty: No such file ")
        .stderr_contains("incorrect: No such file ");

    at.touch("empty");
    at.touch("incorrect");

    // exists but incorrect
    scene
        .ucmd()
        .arg("--check")
        .arg("empty")
        .arg("incorrect")
        .fails()
        .stderr_contains("empty: no properly ")
        .stderr_contains("incorrect: no properly ");
}

#[test]
fn test_check_unknown_checksum_file() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .arg("--check")
        .arg("missing")
        .fails()
        .stderr_only("cksum: missing: No such file or directory\n");
}

#[test]
fn test_check_comment_line() {
    // A comment in a checksum file shall be discarded unnoticed.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write(
        "CHECKSUM-sha1",
        "\
        # This is a comment\n\
        SHA1 (foo) = 058ab38dd3603703b3a7063cf95dc51a4286b6fe\n\
        # next comment is empty\n#",
    );

    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM-sha1")
        .succeeds()
        .stdout_contains("foo: OK")
        .no_stderr();
}

#[test]
fn test_check_comment_only() {
    // A file only filled with comments is equivalent to an empty file,
    // and therefore produces an error.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("CHECKSUM", "# This is a comment\n");

    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .fails()
        .stderr_contains("no properly formatted checksum lines found");
}

#[test]
fn test_check_comment_leading_space() {
    // A comment must have its '#' in first position on the line.
    // A space before it will raise a warning for improperly formatted line.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write(
        "CHECKSUM-sha1",
        " # This is a comment\n\
        SHA1 (foo) = 058ab38dd3603703b3a7063cf95dc51a4286b6fe\n",
    );

    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM-sha1")
        .succeeds()
        .stdout_contains("foo: OK")
        .stderr_contains("WARNING: 1 line is improperly formatted");
}

/// This test checks alignment with GNU's error handling.
/// Windows has a different logic and is guarded by [`test_check_directory_error`].
#[cfg(not(windows))]
#[test]
fn test_check_failed_to_read() {
    // check `cksum`'s behavior when encountering directories or non existing files

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write(
        "CHECKSUM",
        "SHA1 (dir) = ffffffffffffffffffffffffffffffffffffffff\n\
        SHA1 (not-file) = ffffffffffffffffffffffffffffffffffffffff\n",
    );
    at.mkdir("dir");

    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .fails()
        .stdout_is(
            "dir: FAILED open or read\n\
            not-file: FAILED open or read\n",
        )
        .stderr_contains("cksum: WARNING: 2 listed files could not be read");

    // check with `--ignore-missing`
    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .arg("--ignore-missing")
        .fails()
        .stdout_is("dir: FAILED open or read\n")
        .stderr_contains("cksum: WARNING: 1 listed file could not be read");
}

#[test]
fn test_zero_multiple_file() {
    new_ucmd!()
        .arg("-z")
        .arg("alice_in_wonderland.txt")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("zero_multiple_file.expected");
}

#[test]
fn test_zero_single_file() {
    new_ucmd!()
        .arg("--zero")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("zero_single_file.expected");
}

#[test]
fn test_check_trailing_space_fails() {
    // If a checksum line has trailing spaces after the digest,
    // it shall be considered improperly formatted.

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo", "foo-content\n");
    at.write(
        "CHECKSUM",
        "SHA1 (foo) = 058ab38dd3603703b3a7063cf95dc51a4286b6fe    \n",
    );

    scene
        .ucmd()
        .arg("--check")
        .arg("CHECKSUM")
        .fails()
        .no_stdout()
        .stderr_contains("CHECKSUM: no properly formatted checksum lines found");
}

/// Regroup tests related to the handling of non-utf-8 content
/// in checksum files.
/// These tests are excluded from Windows because it does not provide any safe
/// conversion between `OsString` and byte sequences for non-utf-8 strings.
mod check_encoding {

    // This test should pass on linux and macos.
    #[cfg(not(windows))]
    #[test]
    fn test_check_non_utf8_comment() {
        use super::*;
        let hashes =
        b"MD5 (empty) = 1B2M2Y8AsgTpgAmY7PhCfg==\n\
        # Comment with a non utf8 char: >>\xff<<\n\
        SHA256 (empty) = 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\n\
        BLAKE2b (empty) = eGoC90IBWQPGxv2FJVLScpEvR0DhWEdhiobiF/cfVBnSXhAxr+5YUxOJZESTTrBLkDpoWxRIt1XVb3Aa/pvizg==\n"
    ;

        let (at, mut cmd) = at_and_ucmd!();

        at.touch("empty");
        at.write_bytes("check", hashes);

        cmd.arg("--check")
            .arg(at.subdir.join("check"))
            .succeeds()
            .stdout_is("empty: OK\nempty: OK\nempty: OK\n")
            .no_stderr();
    }

    // This test should pass on linux. Windows and macos will fail to
    // create a file which name contains '\xff'.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_check_non_utf8_filename() {
        use super::*;
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        let filename: OsString = OsStringExt::from_vec(b"funky\xffname".to_vec());
        at.touch(filename);

        // Checksum match
        at.write_bytes("check",
            b"SHA256 (funky\xffname) = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n");
        scene
            .ucmd()
            .arg("--check")
            .arg(at.subdir.join("check"))
            .succeeds()
            .stdout_is_bytes(b"funky\xffname: OK\n")
            .no_stderr();

        // Checksum mismatch
        at.write_bytes("check",
            b"SHA256 (funky\xffname) = ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n");
        scene
            .ucmd()
            .arg("--check")
            .arg(at.subdir.join("check"))
            .fails()
            .stdout_is_bytes(b"funky\xffname: FAILED\n")
            .stderr_contains("1 computed checksum did NOT match");

        // file not found
        at.write_bytes("check",
            b"SHA256 (flakey\xffname) = ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n");
        scene
            .ucmd()
            .arg("--check")
            .arg(at.subdir.join("check"))
            .fails()
            .stdout_is_bytes(b"flakey\xffname: FAILED open or read\n")
            .stderr_contains("1 listed file could not be read");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_quoting_in_stderr() {
        use super::*;
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let (at, mut cmd) = at_and_ucmd!();

        at.mkdir(<OsStr as OsStrExt>::from_bytes(b"FFF\xffDIR"));
        at.write_bytes(
            "check",
            b"SHA256 (FFF\xffFFF) = 29953405eaa3dcc41c37d1621d55b6a47eee93e05613e439e73295029740b10c\nSHA256 (FFF\xffDIR) = 29953405eaa3dcc41c37d1621d55b6a47eee93e05613e439e73295029740b10c\n",
        );

        cmd.arg("-c")
            .arg("check")
            .fails_with_code(1)
            .stdout_contains_bytes(b"FFF\xffFFF: FAILED open or read")
            .stdout_contains_bytes(b"FFF\xffDIR: FAILED open or read")
            .stderr_contains("'FFF'$'\\377''FFF': No such file or directory")
            .stderr_contains("'FFF'$'\\377''DIR': Is a directory");
    }
}

#[test]
fn test_check_blake_length_guess() {
    let correct_lines = [
        // Correct: The length is not explicit, but the checksum's size
        // matches the default parameter.
        "BLAKE2b (foo.dat) = ca002330e69d3e6b84a46a56a6533fd79d51d97a3bb7cad6c2ff43b354185d6dc1e723fb3db4ae0737e120378424c714bb982d9dc5bbd7a0ab318240ddd18f8d",
        // Correct: The length is explicitly given, and the checksum's size
        // matches the length.
        "BLAKE2b-512 (foo.dat) = ca002330e69d3e6b84a46a56a6533fd79d51d97a3bb7cad6c2ff43b354185d6dc1e723fb3db4ae0737e120378424c714bb982d9dc5bbd7a0ab318240ddd18f8d",
        // Correct: the checksum size is not default but
        // the length is explicitly given.
        "BLAKE2b-48 (foo.dat) = 171cdfdf84ed",
    ];
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo.dat", "foo");

    for line in correct_lines {
        at.write("foo.sums", line);
        scene
            .ucmd()
            .arg("--check")
            .arg(at.subdir.join("foo.sums"))
            .succeeds()
            .stdout_is("foo.dat: OK\n");
    }

    // Incorrect lines

    // This is incorrect because the algorithm provides no length,
    // and the checksum length is not default.
    let incorrect = "BLAKE2b (foo.dat) = 171cdfdf84ed";
    at.write("foo.sums", incorrect);
    scene
        .ucmd()
        .arg("--check")
        .arg(at.subdir.join("foo.sums"))
        .fails()
        .stderr_contains("foo.sums: no properly formatted checksum lines found");
}

#[test]
fn test_check_confusing_base64() {
    let cksum = "BLAKE2b-48 (foo.dat) = fc1f97C4";

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo.dat", "esq");
    at.write("foo.sums", cksum);

    scene
        .ucmd()
        .arg("--check")
        .arg(at.subdir.join("foo.sums"))
        .succeeds()
        .stdout_is("foo.dat: OK\n");
}

/// This test checks that when a file contains several checksum lines
/// with different encoding, the decoding still works.
#[test]
fn test_check_mix_hex_base64() {
    let b64 = "BLAKE2b-128 (foo1.dat) = BBNuJPhdRwRlw9tm5Y7VbA==";
    let hex = "BLAKE2b-128 (foo2.dat) = 04136e24f85d470465c3db66e58ed56c";

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foo1.dat", "foo");
    at.write("foo2.dat", "foo");

    at.write("hex_b64", &format!("{hex}\n{b64}"));
    at.write("b64_hex", &format!("{b64}\n{hex}"));

    scene
        .ucmd()
        .arg("--check")
        .arg(at.subdir.join("hex_b64"))
        .succeeds()
        .stdout_only("foo2.dat: OK\nfoo1.dat: OK\n");

    scene
        .ucmd()
        .arg("--check")
        .arg(at.subdir.join("b64_hex"))
        .succeeds()
        .stdout_only("foo1.dat: OK\nfoo2.dat: OK\n");
}

/// This test ensures that an improperly formatted base64 checksum in a file
/// does not interrupt the processing of next lines.
#[test]
fn test_check_incorrectly_formatted_checksum_keeps_processing_b64() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");

    let good_ck = "MD5 (f) = 1B2M2Y8AsgTpgAmY7PhCfg=="; // OK
    let bad_ck = "MD5 (f) = 1B2M2Y8AsgTpgAmY7PhCfg="; // Missing last '='

    // Good then Bad
    scene
        .ucmd()
        .arg("--check")
        .pipe_in([good_ck, bad_ck].join("\n").as_bytes().to_vec())
        .succeeds()
        .stdout_contains("f: OK")
        .stderr_contains("cksum: WARNING: 1 line is improperly formatted");

    // Bad then Good
    scene
        .ucmd()
        .arg("--check")
        .pipe_in([bad_ck, good_ck].join("\n").as_bytes().to_vec())
        .succeeds()
        .stdout_contains("f: OK")
        .stderr_contains("cksum: WARNING: 1 line is improperly formatted");
}

/// This test ensures that an improperly formatted hexadecimal checksum in a
/// file does not interrupt the processing of next lines.
#[test]
fn test_check_incorrectly_formatted_checksum_keeps_processing_hex() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");

    let good_ck = "MD5 (f) = d41d8cd98f00b204e9800998ecf8427e"; // OK
    let bad_ck = "MD5 (f) = d41d8cd98f00b204e9800998ecf8427"; // Missing last

    // Good then Bad
    scene
        .ucmd()
        .arg("--check")
        .pipe_in([good_ck, bad_ck].join("\n").as_bytes().to_vec())
        .succeeds()
        .stdout_contains("f: OK")
        .stderr_contains("cksum: WARNING: 1 line is improperly formatted");

    // Bad then Good
    scene
        .ucmd()
        .arg("--check")
        .pipe_in([bad_ck, good_ck].join("\n").as_bytes().to_vec())
        .succeeds()
        .stdout_contains("f: OK")
        .stderr_contains("cksum: WARNING: 1 line is improperly formatted");
}

/// This module reimplements the cksum-base64.pl GNU test.
mod gnu_cksum_base64 {
    use super::*;
    use uutests::util::log_info;

    const PAIRS: [(&str, &str); 12] = [
        ("sysv", "0 0 f"),
        ("bsd", "00000     0 f"),
        ("crc", "4294967295 0 f"),
        ("crc32b", "0 0 f"),
        ("md5", "1B2M2Y8AsgTpgAmY7PhCfg=="),
        ("sha1", "2jmj7l5rSw0yVb/vlWAYkK/YBwk="),
        ("sha224", "0UoCjCo6K8lHYQK7KII0xBWisB+CjqYqxbPkLw=="),
        ("sha256", "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="),
        (
            "sha384",
            "OLBgp1GsljhM2TJ+sbHjaiH9txEUvgdDTAzHv2P24donTt6/529l+9Ua0vFImLlb",
        ),
        (
            "sha512",
            "z4PhNX7vuL3xVChQ1m2AB9Yg5AULVxXcg/SpIdNs6c5H0NE8XYXysP+DGNKHfuwvY7kxvUdBeoGlODJ6+SfaPg==",
        ),
        (
            "blake2b",
            "eGoC90IBWQPGxv2FJVLScpEvR0DhWEdhiobiF/cfVBnSXhAxr+5YUxOJZESTTrBLkDpoWxRIt1XVb3Aa/pvizg==",
        ),
        ("sm3", "GrIdg1XPoX+OYRlIMegajyK+yMco/vt0ftA161CCqis="),
    ];

    fn make_scene() -> TestScenario {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.touch("f");

        scene
    }

    fn output_format(algo: &str, digest: &str) -> String {
        if ["sysv", "bsd", "crc", "crc32b"].contains(&algo) {
            digest.to_string()
        } else {
            format!("{} (f) = {digest}", algo.to_uppercase()).replace("BLAKE2B", "BLAKE2b")
        }
    }

    #[test]
    fn test_generating() {
        // Ensure that each algorithm works with `--base64`.
        let scene = make_scene();

        for (algo, digest) in PAIRS {
            log_info("ALGORITHM", algo);
            scene
                .ucmd()
                .arg("--base64")
                .arg("-a")
                .arg(algo)
                .arg("f")
                .succeeds()
                .stdout_only(format!("{}\n", output_format(algo, digest)));
        }
    }

    #[test]
    fn test_chk() {
        // For each algorithm that accepts `--check`,
        // ensure that it works with base64 digests.
        let scene = make_scene();

        for (algo, digest) in PAIRS {
            if ["sysv", "bsd", "crc", "crc32b"].contains(&algo) {
                // These algorithms do not accept `--check`
                scene
                    .ucmd()
                    .arg("--check")
                    .arg("-a")
                    .arg(algo)
                    .fails()
                    .stderr_only(
                        "cksum: --check is not supported with --algorithm={bsd,sysv,crc,crc32b}\n",
                    );
                continue;
            }

            let line = output_format(algo, digest);
            scene
                .ucmd()
                .arg("--check")
                .arg("--strict")
                .pipe_in(line)
                .succeeds()
                .stdout_only("f: OK\n");
        }
    }

    #[test]
    fn test_chk_eq1() {
        // For digests ending with '=', ensure `--check` fails if '=' is removed.
        let scene = make_scene();

        for (algo, digest) in PAIRS {
            if !digest.ends_with('=') {
                continue;
            }

            let mut line = output_format(algo, digest);
            if line.ends_with('=') {
                line.pop();
            }

            log_info(format!("ALGORITHM: {algo}, STDIN: '{line}'"), "");
            scene
                .ucmd()
                .arg("--check")
                .pipe_in(line)
                .fails()
                .no_stdout()
                .stderr_contains("no properly formatted checksum lines found");
        }
    }

    #[test]
    fn test_chk_eq2() {
        // For digests ending with '==',
        // ensure `--check` fails if '==' is removed.
        let scene = make_scene();

        for (algo, digest) in PAIRS {
            if !digest.ends_with("==") {
                continue;
            }

            let line = output_format(algo, digest);
            let line = line.trim_end_matches("==");

            log_info(format!("ALGORITHM: {algo}, STDIN: '{line}'"), "");
            scene
                .ucmd()
                .arg("--check")
                .pipe_in(line)
                .fails()
                .no_stdout()
                .stderr_contains("no properly formatted checksum lines found");
        }
    }
}

/// This module reimplements the cksum-c.sh GNU test.
mod gnu_cksum_c {
    use super::*;

    const INVALID_SUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaafdb57c725157cb40b5aee8d937b8351477e";

    fn make_scene() -> TestScenario {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.write("input", "9\n7\n1\n4\n2\n6\n3\n5\n8\n10\n");

        let algos: &[&[&str]] = &[
            &["-a", "sha384"],
            &["-a", "blake2b"],
            &["-a", "blake2b", "-l", "384"],
            &["-a", "sm3"],
        ];

        for args in algos {
            let result = scene.ucmd().args(args).succeeds();
            let stdout = result.stdout();
            at.append_bytes("CHECKSUMS", stdout);
        }

        scene
    }

    #[test]
    #[ignore = "todo"]
    fn test_signed_checksums() {
        todo!()
    }

    #[test]
    fn test_check_individual_digests_in_mixed_file() {
        let scene = make_scene();

        scene
            .ucmd()
            .arg("--check")
            .arg("-a")
            .arg("sm3")
            .arg("CHECKSUMS")
            .succeeds();
    }

    #[test]
    fn test_check_against_older_non_hex_formats() {
        let scene = make_scene();

        scene
            .ucmd()
            .arg("-c")
            .arg("-a")
            .arg("crc")
            .arg("CHECKSUMS")
            .fails();

        let crc_cmd = scene.ucmd().arg("-a").arg("crc").arg("input").succeeds();
        let crc_cmd_out = crc_cmd.stdout();
        scene.fixtures.write_bytes("CHECKSUMS.crc", crc_cmd_out);

        scene.ucmd().arg("-c").arg("CHECKSUMS.crc").fails();
    }

    #[test]
    fn test_status() {
        let scene = make_scene();

        scene
            .ucmd()
            .arg("--status")
            .arg("--check")
            .arg("CHECKSUMS")
            .succeeds()
            .no_output();
    }

    fn make_scene_with_comment() -> TestScenario {
        let scene = make_scene();

        scene
            .fixtures
            .append("CHECKSUMS", "# Very important comment\n");

        scene
    }

    #[test]
    fn test_status_with_comment() {
        let scene = make_scene_with_comment();

        scene
            .ucmd()
            .arg("--status")
            .arg("--check")
            .arg("CHECKSUMS")
            .succeeds()
            .no_output();
    }

    fn make_scene_with_invalid_line() -> TestScenario {
        let scene = make_scene_with_comment();

        scene.fixtures.append("CHECKSUMS", "invalid_line\n");

        scene
    }

    #[test]
    fn test_check_strict() {
        let scene = make_scene_with_invalid_line();

        // without strict, succeeds
        scene
            .ucmd()
            .arg("--check")
            .arg("CHECKSUMS")
            .succeeds()
            .stderr_contains("1 line is improperly formatted");

        // with strict, fails
        scene
            .ucmd()
            .arg("--strict")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stderr_contains("1 line is improperly formatted");
    }

    fn make_scene_with_two_invalid_lines() -> TestScenario {
        let scene = make_scene_with_comment();

        scene
            .fixtures
            .append("CHECKSUMS", "invalid_line\ninvalid_line\n");

        scene
    }

    #[test]
    fn test_check_strict_plural_checks() {
        let scene = make_scene_with_two_invalid_lines();

        scene
            .ucmd()
            .arg("--strict")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stderr_contains("2 lines are improperly formatted");
    }

    fn make_scene_with_incorrect_checksum() -> TestScenario {
        let scene = make_scene_with_two_invalid_lines();

        scene
            .fixtures
            .append("CHECKSUMS", &format!("SM3 (input) = {INVALID_SUM}\n"));

        scene
    }

    #[test]
    fn test_check_with_incorrect_checksum() {
        let scene = make_scene_with_incorrect_checksum();

        scene
            .ucmd()
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stdout_contains("input: FAILED")
            .stderr_contains("1 computed checksum did NOT match");

        // also fails with strict
        scene
            .ucmd()
            .arg("--strict")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stdout_contains("input: FAILED")
            .stderr_contains("1 computed checksum did NOT match");
    }

    #[test]
    fn test_status_with_errors() {
        let scene = make_scene_with_incorrect_checksum();

        scene
            .ucmd()
            .arg("--status")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .no_output();
    }

    #[test]
    fn test_check_with_non_existing_file() {
        let scene = make_scene();
        scene
            .fixtures
            .write("CHECKSUMS2", &format!("SM3 (input2) = {INVALID_SUM}\n"));

        scene
            .ucmd()
            .arg("--check")
            .arg("CHECKSUMS2")
            .fails()
            .stdout_contains("input2: FAILED open or read")
            .stderr_contains("1 listed file could not be read");

        // also fails with strict
        scene
            .ucmd()
            .arg("--strict")
            .arg("--check")
            .arg("CHECKSUMS2")
            .fails()
            .stdout_contains("input2: FAILED open or read")
            .stderr_contains("1 listed file could not be read");
    }

    fn make_scene_with_another_improperly_formatted() -> TestScenario {
        let scene = make_scene_with_incorrect_checksum();

        scene.fixtures.append(
            "CHECKSUMS",
            &format!("BLAKE2b (missing-file) = {INVALID_SUM}\n"),
        );

        scene
    }

    #[test]
    fn test_warn() {
        let scene = make_scene_with_another_improperly_formatted();

        scene
            .ucmd()
            .arg("--warn")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stderr_contains("CHECKSUMS: 6: improperly formatted SM3 checksum line")
            .stderr_contains("CHECKSUMS: 9: improperly formatted BLAKE2b checksum line");
    }

    fn make_scene_with_checksum_missing() -> TestScenario {
        let scene = make_scene_with_another_improperly_formatted();

        scene.fixtures.write(
            "CHECKSUMS-missing",
            &format!("SM3 (nonexistent) = {INVALID_SUM}\n"),
        );

        scene
    }

    #[test]
    fn test_ignore_missing() {
        let scene = make_scene_with_checksum_missing();

        scene
            .ucmd()
            .arg("--ignore-missing")
            .arg("--check")
            .arg("CHECKSUMS-missing")
            .fails()
            .stdout_does_not_contain("nonexistent: No such file or directory")
            .stdout_does_not_contain("nonexistent: FAILED open or read")
            .stderr_contains("CHECKSUMS-missing: no file was verified");
    }

    #[test]
    fn test_status_and_warn() {
        let scene = make_scene_with_checksum_missing();

        // --status before --warn
        scene
            .ucmd()
            .arg("--status")
            .arg("--warn")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stderr_contains("CHECKSUMS: 9: improperly formatted BLAKE2b checksum line")
            .stderr_contains("WARNING: 3 lines are improperly formatted")
            .stderr_contains("WARNING: 1 computed checksum did NOT match");

        // --warn before --status (status hides the results)
        scene
            .ucmd()
            .arg("--warn")
            .arg("--status")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .stderr_does_not_contain("CHECKSUMS: 9: improperly formatted BLAKE2b checksum line")
            .stderr_does_not_contain("WARNING: 3 lines are improperly formatted")
            .stderr_does_not_contain("WARNING: 1 computed checksum did NOT match");
    }

    #[test]
    fn test_status_and_ignore_missing() {
        let scene = make_scene_with_checksum_missing();

        scene
            .ucmd()
            .arg("--status")
            .arg("--ignore-missing")
            .arg("--check")
            .arg("CHECKSUMS")
            .fails()
            .no_output();
    }

    #[test]
    fn test_status_warn_and_ignore_missing() {
        let scene = make_scene_with_checksum_missing();

        scene
            .ucmd()
            .arg("--status")
            .arg("--warn")
            .arg("--ignore-missing")
            .arg("--check")
            .arg("CHECKSUMS-missing")
            .fails()
            .stderr_contains("CHECKSUMS-missing: no file was verified")
            .stdout_does_not_contain("nonexistent: No such file or directory");
    }

    #[test]
    fn test_check_several_files_dont_exist() {
        let scene = make_scene();

        scene
            .ucmd()
            .arg("--check")
            .arg("non-existing-1")
            .arg("non-existing-2")
            .fails()
            .stderr_contains("non-existing-1: No such file or directory")
            .stderr_contains("non-existing-2: No such file or directory");
    }

    #[test]
    fn test_check_several_files_empty() {
        let scene = make_scene();
        scene.fixtures.touch("empty-1");
        scene.fixtures.touch("empty-2");

        scene
            .ucmd()
            .arg("--check")
            .arg("empty-1")
            .arg("empty-2")
            .fails()
            .stderr_contains("empty-1: no properly formatted checksum lines found")
            .stderr_contains("empty-2: no properly formatted checksum lines found");
    }
}

/// The tests in this module check the behavior of cksum when given different
/// checksum formats and algorithms in the same file, while specifying an
/// algorithm on CLI or not.
mod format_mix {
    use super::*;

    // First line is algo-based, second one is not
    const INPUT_ALGO_NON_ALGO: &str = "\
        BLAKE2b (bar) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce\n\
        786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce  foo";

    // First line is non algo-based, second one is
    const INPUT_NON_ALGO_ALGO: &str = "\
        786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce  foo\n\
        BLAKE2b (bar) = 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce";

    /// Make a simple scene with foo and bar empty files
    fn make_scene() -> TestScenario {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;

        at.touch("foo");
        at.touch("bar");

        scene
    }

    #[test]
    fn test_check_cli_algo_non_algo() {
        let scene = make_scene();
        scene
            .ucmd()
            .arg("--check")
            .arg("--algo=blake2b")
            .pipe_in(INPUT_ALGO_NON_ALGO)
            .succeeds()
            .stdout_contains("bar: OK\nfoo: OK")
            .no_stderr();
    }

    #[test]
    fn test_check_cli_non_algo_algo() {
        let scene = make_scene();
        scene
            .ucmd()
            .arg("--check")
            .arg("--algo=blake2b")
            .pipe_in(INPUT_NON_ALGO_ALGO)
            .succeeds()
            .stdout_contains("foo: OK\nbar: OK")
            .no_stderr();
    }

    #[test]
    fn test_check_algo_non_algo() {
        let scene = make_scene();
        scene
            .ucmd()
            .arg("--check")
            .pipe_in(INPUT_ALGO_NON_ALGO)
            .succeeds()
            .stdout_contains("bar: OK")
            .stderr_contains("cksum: WARNING: 1 line is improperly formatted");
    }

    #[test]
    fn test_check_non_algo_algo() {
        let scene = make_scene();
        scene
            .ucmd()
            .arg("--check")
            .pipe_in(INPUT_NON_ALGO_ALGO)
            .succeeds()
            .stdout_contains("bar: OK")
            .stderr_contains("cksum: WARNING: 1 line is improperly formatted");
    }
}
