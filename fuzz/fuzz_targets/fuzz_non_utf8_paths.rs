// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![no_main]
use libfuzzer_sys::fuzz_target;
use rand::prelude::IndexedRandom;
use std::env::temp_dir;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use uufuzz::{run_gnu_cmd, CommandResult};
// Programs that typically take file/path arguments and should be tested
static PATH_PROGRAMS: &[&str] = &[
    "basename", "cat", "chmod", "cp", "dirname", "du", "head", "ln", "ls", "mkdir", "mv",
    "readlink", "realpath", "rm", "rmdir", "tail", "touch", "unlink",
];

fn generate_non_utf8_bytes() -> Vec<u8> {
    let mut rng = rand::rng();
    let mut bytes = Vec::new();

    // Start with some valid UTF-8 to make it look like a reasonable path
    bytes.extend_from_slice(b"test_");

    // Add some invalid UTF-8 sequences
    match rng.random_range(0..4) {
        0 => bytes.extend_from_slice(&[0xFF, 0xFE]), // Invalid UTF-8
        1 => bytes.extend_from_slice(&[0xC0, 0x80]), // Overlong encoding
        2 => bytes.extend_from_slice(&[0xED, 0xA0, 0x80]), // UTF-16 surrogate
        _ => bytes.extend_from_slice(&[0xF4, 0x90, 0x80, 0x80]), // Beyond Unicode range
    }

    bytes
}

fn generate_non_utf8_osstring() -> OsString {
    OsString::from_vec(generate_non_utf8_bytes())
}

fn setup_test_files() -> Result<(PathBuf, Vec<PathBuf>), std::io::Error> {
    let mut rng = rand::rng();
    let temp_root = temp_dir().join(format!("utf8_test_{}", rng.random::<u64>()));
    fs::create_dir_all(&temp_root)?;

    let mut test_files = Vec::new();

    // Create some files with non-UTF-8 names
    for i in 0..3 {
        let mut path_bytes = temp_root.as_os_str().as_bytes().to_vec();
        path_bytes.push(b'/');

        if i == 0 {
            // One normal UTF-8 file for comparison
            path_bytes.extend_from_slice(b"normal_file.txt");
        } else {
            // Files with invalid UTF-8 names
            path_bytes.extend_from_slice(&generate_non_utf8_bytes());
        }

        let file_path = PathBuf::from(OsStr::from_bytes(&path_bytes));

        // Try to create the file - this may fail on some filesystems
        if let Ok(mut file) = fs::File::create(&file_path) {
            use std::io::Write;
            let _ = write!(file, "test content for file {}\n", i);
            test_files.push(file_path);
        }
    }

    Ok((temp_root, test_files))
}

fn test_program_with_non_utf8_path(program: &str, path: &PathBuf) -> CommandResult {
    let path_os = path.as_os_str();
    let args = vec![OsString::from(program), path_os.to_owned()];

    // Try to run the GNU version to compare behavior
    match run_gnu_cmd(program, &args[1..], false, None) {
        Ok(result) => result,
        Err(error_result) => {
            // GNU command failed, return the error
            error_result
        }
    }
}

fn cleanup_test_files(temp_root: &PathBuf) {
    let _ = fs::remove_dir_all(temp_root);
}

fn check_for_utf8_error_and_panic(result: &CommandResult, program: &str, path: &PathBuf) {
    let stderr_lower = result.stderr.to_lowercase();
    let is_utf8_error = stderr_lower.contains("invalid utf-8")
        || stderr_lower.contains("not valid unicode")
        || stderr_lower.contains("invalid utf8")
        || stderr_lower.contains("utf-8 error");

    if is_utf8_error {
        println!(
            "UTF-8 conversion error detected in {}: {}",
            program, result.stderr
        );
        println!("Path: {:?}", path);
        println!("Exit code: {}", result.exit_code);
        panic!(
            "FUZZER FAILURE: {} failed with UTF-8 error on non-UTF-8 path: {:?}",
            program, path
        );
    }
}

fuzz_target!(|_data: &[u8]| {
    let mut rng = rand::rng();

    // Set up test environment
    let (temp_root, test_files) = match setup_test_files() {
        Ok(files) => files,
        Err(_) => return, // Skip if we can't set up test files
    };

    // Pick a random program that works with paths
    let program = PATH_PROGRAMS.choose(&mut rng).unwrap();

    // Test with files that have non-UTF-8 names
    for test_file in &test_files {
        let result = test_program_with_non_utf8_path(program, test_file);

        // Check if the program handled the non-UTF-8 path gracefully
        // This will panic on the first UTF-8 error found
        check_for_utf8_error_and_panic(&result, program, test_file);

        // Special test for chmod since that's what the bug report specifically mentions
        if *program == "chmod" && test_file.to_string_lossy().contains('\u{FFFD}') {
            // This path contains replacement characters, indicating invalid UTF-8
            println!("Testing chmod with non-UTF-8 path: {:?}", test_file);

            // Try chmod with basic permissions
            let chmod_args = vec![
                OsString::from("chmod"),
                OsString::from("644"),
                test_file.as_os_str().to_owned(),
            ];

            let chmod_result = run_gnu_cmd("chmod", &chmod_args[1..], false, None);
            match chmod_result {
                Ok(result) => {
                    check_for_utf8_error_and_panic(&result, "chmod", test_file);
                }
                Err(error) => {
                    check_for_utf8_error_and_panic(&error, "chmod", test_file);
                }
            }
        }
    }

    // Test creating directories with non-UTF-8 names
    if *program == "mkdir" {
        let non_utf8_dir_name = generate_non_utf8_osstring();
        let non_utf8_dir = temp_root.join(non_utf8_dir_name);

        let mkdir_args = vec![OsString::from("mkdir"), non_utf8_dir.as_os_str().to_owned()];

        let mkdir_result = run_gnu_cmd("mkdir", &mkdir_args[1..], false, None);
        match mkdir_result {
            Ok(result) => {
                check_for_utf8_error_and_panic(&result, "mkdir", &non_utf8_dir);
            }
            Err(error) => {
                check_for_utf8_error_and_panic(&error, "mkdir", &non_utf8_dir);
            }
        }
    }

    // Clean up
    cleanup_test_files(&temp_root);
});
