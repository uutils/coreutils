// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore osstring

#![no_main]
use libfuzzer_sys::fuzz_target;
use rand::Rng;
use rand::prelude::IndexedRandom;
use std::collections::HashSet;
use std::env::temp_dir;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use uufuzz::{CommandResult, run_gnu_cmd};
// Programs that typically take file/path arguments and should be tested
static PATH_PROGRAMS: &[&str] = &[
    // Core file operations
    "cat",
    "cp",
    "mv",
    "rm",
    "ln",
    "link",
    "unlink",
    "touch",
    "truncate",
    // Path operations
    "ls",
    "mkdir",
    "rmdir",
    "du",
    "stat",
    "mktemp",
    "df",
    "basename",
    "dirname",
    "readlink",
    "realpath",
    "pathchk",
    "chroot",
    // File processing
    "head",
    "tail",
    "tee",
    "more",
    "od",
    "wc",
    "cksum",
    "sum",
    "nl",
    "tac",
    "sort",
    "uniq",
    "split",
    "csplit",
    "cut",
    "tr",
    "shred",
    "shuf",
    "ptx",
    "tsort",
    // Text processing with files
    "chmod",
    "chown",
    "chgrp",
    "install",
    "chcon",
    "runcon",
    "comm",
    "join",
    "paste",
    "pr",
    "fmt",
    "fold",
    "expand",
    "unexpand",
    "dir",
    "vdir",
    "mkfifo",
    "mknod",
    "hashsum",
    // File I/O utilities
    "dd",
    "sync",
    "stdbuf",
    "dircolors",
    // Encoding/decoding utilities
    "base32",
    "base64",
    "basenc",
    "stty",
    "tty",
    "env",
    "nohup",
    "nice",
    "timeout",
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

    // Use the locally built uutils binary instead of system PATH
    let local_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
        .unwrap_or_else(|_| "target/release/coreutils".to_string());

    // Build appropriate arguments for each program
    let local_args = match program {
        // Programs that need mode/permissions
        "chmod" => vec![
            OsString::from(program),
            OsString::from("644"),
            path_os.to_owned(),
        ],
        "chown" => vec![
            OsString::from(program),
            OsString::from("root:root"),
            path_os.to_owned(),
        ],
        "chgrp" => vec![
            OsString::from(program),
            OsString::from("root"),
            path_os.to_owned(),
        ],
        "chcon" => vec![
            OsString::from(program),
            OsString::from("system_u:object_r:admin_home_t:s0"),
            path_os.to_owned(),
        ],
        "runcon" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from("system_u:object_r:admin_home_t:s0"),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        // Programs that need source and destination
        "cp" | "mv" | "ln" | "link" => {
            let dest_path = path.with_extension("dest");
            vec![
                OsString::from(program),
                path_os.to_owned(),
                dest_path.as_os_str().to_owned(),
            ]
        }
        "install" => {
            let dest_path = path.with_extension("dest");
            vec![
                OsString::from(program),
                path_os.to_owned(),
                dest_path.as_os_str().to_owned(),
            ]
        }
        // Programs that need size/truncate operations
        "truncate" => vec![
            OsString::from(program),
            OsString::from("--size=0"),
            path_os.to_owned(),
        ],
        "split" => vec![
            OsString::from(program),
            path_os.to_owned(),
            OsString::from("split_prefix_"),
        ],
        "csplit" => vec![
            OsString::from(program),
            path_os.to_owned(),
            OsString::from("1"),
        ],
        // File creation programs
        "mkfifo" | "mknod" => {
            let new_path = path.with_extension("new");
            if program == "mknod" {
                vec![
                    OsString::from(program),
                    new_path.as_os_str().to_owned(),
                    OsString::from("c"),
                    OsString::from("1"),
                    OsString::from("3"),
                ]
            } else {
                vec![OsString::from(program), new_path.as_os_str().to_owned()]
            }
        }
        "dd" => vec![
            OsString::from(program),
            OsString::from(format!("if={}", path_os.to_string_lossy())),
            OsString::from("of=/dev/null"),
            OsString::from("bs=1"),
            OsString::from("count=1"),
        ],
        // Hashsum needs algorithm
        "hashsum" => vec![
            OsString::from(program),
            OsString::from("--md5"),
            path_os.to_owned(),
        ],
        // Encoding/decoding programs
        "base32" | "base64" | "basenc" => vec![OsString::from(program), path_os.to_owned()],
        "df" => vec![OsString::from(program), path_os.to_owned()],
        "chroot" => {
            // chroot needs a directory and command
            vec![
                OsString::from(program),
                path_os.to_owned(),
                OsString::from("true"),
            ]
        }
        "sync" => vec![OsString::from(program), path_os.to_owned()],
        "stty" => vec![
            OsString::from(program),
            OsString::from("-F"),
            path_os.to_owned(),
        ],
        "tty" => vec![OsString::from(program)], // tty doesn't take file args, but test anyway
        "env" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        "nohup" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        "nice" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        "timeout" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from("1"),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        "stdbuf" => {
            let coreutils_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            vec![
                OsString::from(program),
                OsString::from("-o0"),
                OsString::from(coreutils_binary),
                OsString::from("cat"),
                path_os.to_owned(),
            ]
        }
        // Programs that work with multiple files (use just one for testing)
        "comm" | "join" => {
            // These need two files, use the same file twice for simplicity
            vec![
                OsString::from(program),
                path_os.to_owned(),
                path_os.to_owned(),
            ]
        }
        // Programs that typically take file input
        _ => vec![OsString::from(program), path_os.to_owned()],
    };

    // Try to run the local uutils version
    match run_gnu_cmd(&local_binary, &local_args, false, None) {
        Ok(result) => result,
        Err(error_result) => {
            // Local command failed, return the error
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

    // Pick multiple random programs to test in each iteration
    let num_programs_to_test = rng.random_range(1..=3); // Test 1-3 programs per iteration
    let mut tested_programs = HashSet::new();

    let mut programs_tested = Vec::<String>::new();

    for _ in 0..num_programs_to_test {
        // Pick a random program that we haven't tested yet in this iteration
        let available_programs: Vec<_> = PATH_PROGRAMS
            .iter()
            .filter(|p| !tested_programs.contains(*p))
            .collect();

        if available_programs.is_empty() {
            break;
        }

        let program = available_programs.choose(&mut rng).unwrap();
        tested_programs.insert(*program);
        programs_tested.push(program.to_string());

        // Test with one random file that has non-UTF-8 names (not all files to speed up)
        if let Some(test_file) = test_files.choose(&mut rng) {
            let result = test_program_with_non_utf8_path(program, test_file);

            // Check if the program handled the non-UTF-8 path gracefully
            check_for_utf8_error_and_panic(&result, program, test_file);
        }

        // Special cases for programs that need additional testing
        if **program == "mkdir" || **program == "mktemp" {
            let non_utf8_dir_name = generate_non_utf8_osstring();
            let non_utf8_dir = temp_root.join(non_utf8_dir_name);

            let local_binary = std::env::var("CARGO_BIN_FILE_COREUTILS")
                .unwrap_or_else(|_| "target/release/coreutils".to_string());
            let mkdir_args = vec![OsString::from("mkdir"), non_utf8_dir.as_os_str().to_owned()];

            let mkdir_result = run_gnu_cmd(&local_binary, &mkdir_args, false, None);
            match mkdir_result {
                Ok(result) => {
                    check_for_utf8_error_and_panic(&result, "mkdir", &non_utf8_dir);
                }
                Err(error) => {
                    check_for_utf8_error_and_panic(&error, "mkdir", &non_utf8_dir);
                }
            }
        }
    }

    println!("Tested programs: {}", programs_tested.join(", "));

    // Clean up
    cleanup_test_files(&temp_root);
});
