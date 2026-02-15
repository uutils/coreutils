// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_dirname::uumain;

use rand::Rng;
use rand::prelude::IndexedRandom;
use std::ffi::OsString;

use uufuzz::CommandResult;
use uufuzz::{compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd};

static CMD_PATH: &str = "dirname";

fn generate_dirname_args() -> Vec<String> {
    let mut rng = rand::rng();
    let mut args = Vec::new();

    // 20% chance to include -z/--zero flag
    if rng.random_bool(0.2) {
        if rng.random_bool(0.5) {
            args.push("-z".to_owned());
        } else {
            args.push("--zero".to_owned());
        }
    }

    // 30% chance to use one of the specific issue #8924 cases
    if rng.random_bool(0.3) {
        let issue_cases = [
            "foo//.",
            "foo/./",
            "foo/bar/./",
            "bar//.",
            "test/./",
            "a/b/./",
            "x//.",
            "dir/subdir/./",
        ];
        args.push(issue_cases.choose(&mut rng).unwrap().to_string());
    } else {
        // Generate 1-3 path arguments normally
        let num_paths = rng.random_range(1..=3);
        for _ in 0..num_paths {
            args.push(generate_path());
        }
    }

    args
}

fn generate_path() -> String {
    let mut rng = rand::rng();

    // Different types of paths to test
    let path_type = rng.random_range(0..15);

    match path_type {
        // Simple paths
        0 => generate_random_string(rng.random_range(1..=20)),

        // Paths with slashes
        1 => {
            let mut path = String::new();
            let components = rng.random_range(1..=5);
            for i in 0..components {
                if i > 0 {
                    path.push('/');
                }
                path.push_str(&generate_random_string(rng.random_range(1..=10)));
            }
            path
        }

        // Root path
        2 => "/".to_owned(),

        // Absolute paths
        3 => {
            let mut path = "/".to_owned();
            let components = rng.random_range(1..=4);
            for _ in 0..components {
                path.push_str(&generate_random_string(rng.random_range(1..=8)));
                path.push('/');
            }
            // Remove trailing slash sometimes
            if rng.random_bool(0.5) && path.len() > 1 {
                path.pop();
            }
            path
        }

        // Paths ending with "/." (specific case from issue #8924)
        4 => {
            let base = if rng.random_bool(0.3) {
                "/".to_owned()
            } else {
                format!("/{}", generate_random_string(rng.random_range(1..=10)))
            };
            format!("{}.", base)
        }

        // Paths with multiple slashes
        5 => {
            let base = generate_random_string(rng.random_range(1..=10));
            format!(
                "///{}//{}",
                base,
                generate_random_string(rng.random_range(1..=8))
            )
        }

        // Paths with dots
        6 => {
            let components = [".", "..", "...", "...."];
            let chosen = components.choose(&mut rng).unwrap();
            if rng.random_bool(0.5) {
                format!("/{}", chosen)
            } else {
                chosen.to_string()
            }
        }

        // Single character paths
        7 => {
            let chars = ['a', 'x', '1', '-', '_', '.'];
            chars.choose(&mut rng).unwrap().to_string()
        }

        // Empty string (edge case)
        8 => "".to_owned(),

        // Issue #8924 specific cases: paths like "foo//."
        9 => {
            let base = generate_random_string(rng.random_range(1..=10));
            format!("{}//.", base)
        }

        // Issue #8924 specific cases: paths like "foo/./"
        10 => {
            let base = generate_random_string(rng.random_range(1..=10));
            format!("{}/./", base)
        }

        // Issue #8924 specific cases: paths like "foo/bar/./"
        11 => {
            let base1 = generate_random_string(rng.random_range(1..=8));
            let base2 = generate_random_string(rng.random_range(1..=8));
            format!("{}/{}/./", base1, base2)
        }

        // More complex patterns with ./ and multiple slashes
        12 => {
            let base = generate_random_string(rng.random_range(1..=10));
            let patterns = ["/./", "//./", "//.//", "/.//"];
            let pattern = patterns.choose(&mut rng).unwrap();
            format!("{}{}", base, pattern)
        }

        // Patterns with .. and multiple slashes
        13 => {
            let base = generate_random_string(rng.random_range(1..=10));
            let patterns = ["/..", "//..", "/../", "//..//"];
            let pattern = patterns.choose(&mut rng).unwrap();
            format!("{}{}", base, pattern)
        }

        // Complex paths with special cases
        _ => {
            let special_endings = [".", "..", "/.", "/..", "//", "/", "/./.", "//.", "./"];
            let base = generate_random_string(rng.random_range(1..=15));
            let ending = special_endings.choose(&mut rng).unwrap();
            format!("{}{}", base, ending)
        }
    }
}

fuzz_target!(|_data: &[u8]| {
    let dirname_args = generate_dirname_args();
    let mut args = vec![OsString::from("dirname")];
    args.extend(dirname_args.iter().map(OsString::from));

    let rust_result = generate_and_run_uumain(&args, uumain, None);

    let gnu_result = match run_gnu_cmd(CMD_PATH, &args[1..], false, None) {
        Ok(result) => result,
        Err(error_result) => {
            eprintln!("Failed to run GNU command:");
            eprintln!("Stderr: {}", error_result.stderr);
            eprintln!("Exit Code: {}", error_result.exit_code);
            CommandResult {
                stdout: String::new(),
                stderr: error_result.stderr,
                exit_code: error_result.exit_code,
            }
        }
    };

    compare_result(
        "dirname",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false,
    );
});
