// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::common::util::TestScenario;

#[test]
fn test_basic_blake2b() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    let result = scene.ucmd().arg("test.txt").succeeds();

    // Blake2b-512 hash of "hello world\n"
    assert!(result.stdout_str().contains("test.txt"));
}

#[test]
fn test_blake2b_stdin() {
    let scene = TestScenario::new(util_name!());

    scene
        .ucmd()
        .pipe_in("hello world\n")
        .succeeds()
        .stdout_contains("  -");
}

#[test]
fn test_blake2b_with_length() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    // Test with 256-bit length
    scene.ucmd().arg("-l").arg("256").arg("test.txt").succeeds();
}

#[test]
fn test_blake2b_tag_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    let result = scene.ucmd().arg("--tag").arg("test.txt").succeeds();

    // BSD-style format: BLAKE2b (test.txt) = hash
    assert!(result.stdout_str().contains("BLAKE2b (test.txt) ="));
}

#[test]
fn test_blake2b_tag_format_with_length() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("f");

    // Test with default length (512)
    scene
        .ucmd()
        .arg("--tag")
        .arg("-l")
        .arg("0")
        .arg("f")
        .succeeds()
        .stdout_contains("BLAKE2b (f) =");

    // Test with 128-bit length
    scene
        .ucmd()
        .arg("--tag")
        .arg("-l")
        .arg("128")
        .arg("f")
        .succeeds()
        .stdout_contains("BLAKE2b-128 (f) =");

    // Test with 256-bit length
    scene
        .ucmd()
        .arg("--tag")
        .arg("-l")
        .arg("256")
        .arg("f")
        .succeeds()
        .stdout_contains("BLAKE2b-256 (f) =");
}

#[test]
fn test_blake2b_binary_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    scene.ucmd().arg("-b").arg("test.txt").succeeds();
}

#[test]
fn test_blake2b_text_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    scene.ucmd().arg("-t").arg("test.txt").succeeds();
}

#[test]
fn test_check_tag_format_with_special_filenames() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create files with special names
    for filename in &["a", " b", "*c", "44", " "] {
        at.write(filename, &format!("{}\n", filename));
    }

    // Generate checksums with --tag for different lengths
    let mut checksum_content = String::new();
    for filename in &["a", " b", "*c", "44", " "] {
        for length in &["0", "128"] {
            let result = scene
                .ucmd()
                .arg("--tag")
                .arg("-l")
                .arg(length)
                .arg(filename)
                .succeeds();
            checksum_content.push_str(result.stdout_str());
        }
    }

    at.write("check.b2sum", &checksum_content);

    // Verify with --strict -c
    scene
        .ucmd()
        .arg("--strict")
        .arg("-c")
        .arg("check.b2sum")
        .succeeds();
}

#[test]
fn test_check_untagged_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    for length in &["0", "128"] {
        let result = scene
            .ucmd()
            .arg("--text")
            .arg("-l")
            .arg(length)
            .arg("empty")
            .succeeds();

        at.write("check.b2sum", result.stdout_str());

        // Check with explicit length
        scene
            .ucmd()
            .arg("-l")
            .arg(length)
            .arg("--strict")
            .arg("-c")
            .arg("check.b2sum")
            .succeeds();

        // Check with inferred length
        scene
            .ucmd()
            .arg("--strict")
            .arg("-c")
            .arg("check.b2sum")
            .succeeds();
    }
}

#[test]
fn test_known_checksum_value() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a file and verify against a known checksum
    // This tests that our implementation produces the same hash as GNU
    at.write("empty", "");

    let result = scene.ucmd().arg("--length=128").arg("empty").succeeds();

    // Empty file should produce this specific 128-bit Blake2b hash
    let stdout = result.stdout_str();
    assert!(stdout.contains("cae66941d9efbd404e4d88758ea67670"));
}

#[test]
fn test_check_malformed_input() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // These malformed check lines should fail gracefully (not segfault)
    at.write(
        "crash.check",
        "BLAKE2\nBLAKE2b\nBLAKE2-\nBLAKE2(\nBLAKE2 (\n",
    );

    scene.ucmd().arg("-c").arg("crash.check").fails();
}

#[test]
fn test_check_overflow() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // This should not cause buffer overflow
    at.write("overflow.check", "0A0BA0\n");

    scene.ucmd().arg("-c").arg("overflow.check").fails();
}

#[test]
fn test_multiple_length_options_last_wins() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    // When multiple -l options are specified, the last one should win
    scene
        .ucmd()
        .arg("-l")
        .arg("123")
        .arg("-l")
        .arg("128")
        .arg("empty")
        .succeeds()
        .stdout_contains("cae66941d9efbd404e4d88758ea67670");
}

#[test]
fn test_invalid_length_too_large() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    for length in &["513", "1024"] {
        scene
            .ucmd()
            .arg("-l")
            .arg(length)
            .arg("empty")
            .fails()
            .no_stdout()
            .stderr_contains(format!("invalid length: '{}'", length))
            .stderr_contains("maximum digest length for 'BLAKE2b' is 512 bits");
    }
}

#[test]
fn test_invalid_length_not_multiple_of_8() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    // Length must be multiple of 8
    scene
        .ucmd()
        .arg("-l")
        .arg("13")
        .arg("test.txt")
        .fails()
        .stderr_contains("invalid length: '13'")
        .stderr_contains("length is not a multiple of 8");

    scene
        .ucmd()
        .arg("-l")
        .arg("9")
        .arg("test.txt")
        .fails()
        .stderr_contains("invalid length: '9'")
        .stderr_contains("length is not a multiple of 8");
}

#[test]
fn test_invalid_length_exceeds_maximum() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("test.txt", "hello world\n");

    // Length exceeds maximum of 512
    scene
        .ucmd()
        .arg("-l")
        .arg("520")
        .arg("test.txt")
        .fails()
        .stderr_contains("invalid length: '520'")
        .stderr_contains("maximum digest length for 'BLAKE2b' is 512 bits");
}

#[test]
fn test_check_valid_checksums() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("file1.txt", "test data 1\n");
    at.write("file2.txt", "test data 2\n");

    // Generate checksums
    let result1 = scene.ucmd().arg("file1.txt").succeeds();
    let result2 = scene.ucmd().arg("file2.txt").succeeds();

    let checksum_file = format!("{}{}", result1.stdout_str(), result2.stdout_str());
    at.write("checksums.b2", &checksum_file);

    // Verify checksums
    scene
        .ucmd()
        .arg("-c")
        .arg("checksums.b2")
        .succeeds()
        .stdout_contains("file1.txt: OK")
        .stdout_contains("file2.txt: OK");
}

#[test]
fn test_check_invalid_checksum() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("file.txt", "test data\n");

    // Create a checksum file with wrong hash
    at.write(
        "bad.b2",
        "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000  file.txt\n"
    );

    scene
        .ucmd()
        .arg("-c")
        .arg("bad.b2")
        .fails()
        .stdout_contains("file.txt: FAILED")
        .stderr_contains("WARNING: 1 computed checksum did NOT match");
}

#[test]
fn test_check_missing_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Checksum file references a file that doesn't exist
    at.write(
        "missing.b2",
        "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000  nonexistent.txt\n"
    );

    scene
        .ucmd()
        .arg("-c")
        .arg("missing.b2")
        .fails()
        .stdout_contains("nonexistent.txt: FAILED open or read");
}

#[test]
fn test_check_with_tag_format() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    // Generate tagged format checksum
    let result = scene.ucmd().arg("--tag").arg("empty").succeeds();

    at.write("tagged.b2", result.stdout_str());

    // Verify tagged format
    scene
        .ucmd()
        .arg("-c")
        .arg("tagged.b2")
        .succeeds()
        .stdout_contains("empty: OK");
}

#[test]
fn test_check_strict_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("file.txt");

    let result = scene.ucmd().arg("file.txt").succeeds();

    // Add some invalid lines to the checksum file
    let mut content = result.stdout_str().to_string();
    content.push_str("invalid line here\n");
    content.push_str("another bad line\n");

    at.write("checksums.b2", &content);

    // Without --strict, should succeed but warn
    scene
        .ucmd()
        .arg("-c")
        .arg("checksums.b2")
        .succeeds()
        .stdout_contains("file.txt: OK")
        .stderr_contains("2 lines are improperly formatted");

    // With --strict, should fail
    scene
        .ucmd()
        .arg("--strict")
        .arg("-c")
        .arg("checksums.b2")
        .fails()
        .stdout_contains("file.txt: OK")
        .stderr_contains("2 lines are improperly formatted");
}

#[test]
fn test_multiple_files() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("file1.txt", "content 1\n");
    at.write("file2.txt", "content 2\n");
    at.write("file3.txt", "content 3\n");

    let result = scene
        .ucmd()
        .arg("file1.txt")
        .arg("file2.txt")
        .arg("file3.txt")
        .succeeds();

    let stdout = result.stdout_str();
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("file3.txt"));
}

#[test]
fn test_empty_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("empty");

    scene
        .ucmd()
        .arg("empty")
        .succeeds()
        .stdout_contains("empty");
}

#[test]
fn test_length_zero_equals_512() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("file");

    // -l 0 should be equivalent to -l 512 (default)
    let result_0 = scene.ucmd().arg("-l").arg("0").arg("file").succeeds();
    let result_512 = scene.ucmd().arg("-l").arg("512").arg("file").succeeds();
    let result_default = scene.ucmd().arg("file").succeeds();

    // All three should produce the same hash
    let hash_0 = result_0.stdout_str().split_whitespace().next().unwrap();
    let hash_512 = result_512.stdout_str().split_whitespace().next().unwrap();
    let hash_default = result_default
        .stdout_str()
        .split_whitespace()
        .next()
        .unwrap();

    assert_eq!(hash_0, hash_512);
    assert_eq!(hash_0, hash_default);
}

#[test]
fn test_openssl_format_compatibility() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("file");

    // Generate normal tagged format: "BLAKE2b (file) = hash"
    let result = scene.ucmd().arg("--tag").arg("file").succeeds();
    let normal_format = result.stdout_str();

    // Create OpenSSL variant: "BLAKE2b(file)=hash" (no spaces)
    let openssl_format = normal_format.replace(" (", "(").replace(") =", ")=");

    at.write("openssl.b2", &openssl_format);

    // Should be able to verify OpenSSL format
    scene
        .ucmd()
        .arg("--strict")
        .arg("-c")
        .arg("openssl.b2")
        .succeeds()
        .stdout_contains("file: OK");
}

#[test]
fn test_binary_data() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Write binary data
    at.write_bytes("binary.dat", &[0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD]);

    scene
        .ucmd()
        .arg("binary.dat")
        .succeeds()
        .stdout_contains("binary.dat");
}

#[test]
fn test_check_quiet_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("file");

    let result = scene.ucmd().arg("file").succeeds();
    at.write("checksums.b2", result.stdout_str());

    // Quiet mode should suppress OK messages
    scene
        .ucmd()
        .arg("--quiet")
        .arg("-c")
        .arg("checksums.b2")
        .succeeds()
        .no_stdout()
        .no_stderr();
}

#[test]
fn test_check_status_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("file");

    let result = scene.ucmd().arg("file").succeeds();
    at.write("checksums.b2", result.stdout_str());

    // Status mode should suppress all output
    scene
        .ucmd()
        .arg("--status")
        .arg("-c")
        .arg("checksums.b2")
        .succeeds()
        .no_stdout()
        .no_stderr();
}
