// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_wc::uumain;

use rand::Rng;
use std::ffi::OsString;

use uufuzz::{
    CommandResult, compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};
static CMD_PATH: &str = "wc";

fn generate_wc_args() -> String {
    let mut rng = rand::rng();
    let arg_count = rng.random_range(1..=6);
    let mut args = Vec::new();

    for _ in 0..arg_count {
        // Introduce a chance to add invalid arguments
        if rng.random_bool(0.1) {
            args.push(generate_random_string(rng.random_range(1..=20)));
        } else {
            match rng.random_range(0..=5) {
                0 => args.push(String::from("-c")),
                1 => args.push(String::from("-m")),
                2 => args.push(String::from("-l")),
                3 => args.push(String::from("-L")),
                4 => args.push(String::from("-w")),
                // TODO
                5 => {
                    args.push(String::from("--files0-from"));
                    if rng.random_bool(0.5) {
                        args.push(generate_random_string(50)); // Longer invalid file name
                    } else {
                        args.push(generate_random_string(5));
                    }
                }
                _ => (),
            }
        }
    }

    args.join(" ")
}

// Function to generate a random string of lines, including invalid ones
fn generate_random_lines(count: usize) -> String {
    let mut rng = rand::rng();
    let mut lines = Vec::new();

    for _ in 0..count {
        if rng.random_bool(0.1) {
            lines.push(generate_random_string(rng.random_range(1000..=5000))); // Very long invalid line
        } else {
            lines.push(generate_random_string(rng.random_range(1..=20)));
        }
    }

    lines.join("\n")
}

fuzz_target!(|_data: &[u8]| {
    let wc_args = generate_wc_args();
    let mut args = vec![OsString::from("wc")];
    args.extend(wc_args.split_whitespace().map(OsString::from));

    let input_lines = generate_random_lines(10);

    let rust_result = generate_and_run_uumain(&args, uumain, Some(&input_lines));
    let gnu_result = match run_gnu_cmd(CMD_PATH, &args[1..], false, Some(&input_lines)) {
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
        "wc",
        &format!("{:?}", &args[1..]),
        Some(&input_lines),
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
