// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_seq::uumain;

use rand::Rng;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};
static CMD_PATH: &str = "seq";

fn generate_seq() -> String {
    let mut rng = rand::thread_rng();

    // Generate 1 to 3 numbers for seq arguments
    let arg_count = rng.gen_range(1..=3);
    let mut args = Vec::new();

    for _ in 0..arg_count {
        if rng.gen_ratio(1, 100) {
            // 1% chance to add a random string
            args.push(generate_random_string(rng.gen_range(1..=10)));
        } else {
            // 99% chance to add a numeric value
            match rng.gen_range(0..=3) {
                0 => args.push(rng.gen_range(-10000..=10000).to_string()), // Large or small integers
                1 => args.push(rng.gen_range(-100.0..100.0).to_string()),  // Floating-point numbers
                2 => args.push(rng.gen_range(-100..0).to_string()),        // Negative integers
                _ => args.push(rng.gen_range(1..=100).to_string()),        // Regular integers
            }
        }
    }

    args.join(" ")
}

fuzz_target!(|_data: &[u8]| {
    let seq = generate_seq();
    let mut args = vec![OsString::from("seq")];
    args.extend(seq.split_whitespace().map(OsString::from));

    let rust_result = generate_and_run_uumain(&args, uumain);

    let gnu_result = match run_gnu_cmd(CMD_PATH, &args[1..], false) {
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
        "seq",
        &format!("{:?}", &args[1..]),
        &rust_result.stdout,
        &gnu_result.stdout,
        &rust_result.stderr,
        &gnu_result.stderr,
        rust_result.exit_code,
        gnu_result.exit_code,
        false, // Set to true if you want to fail on stderr diff
    );
});
