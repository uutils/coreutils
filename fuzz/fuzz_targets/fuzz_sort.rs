// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_sort::uumain;

use rand::Rng;
use std::env;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};
static CMD_PATH: &str = "sort";

fn generate_sort_args() -> String {
    let mut rng = rand::thread_rng();

    let arg_count = rng.gen_range(1..=5);
    let mut args = Vec::new();

    for _ in 0..arg_count {
        match rng.gen_range(0..=4) {
            0 => args.push(String::from("-r")), // Reverse the result of comparisons
            1 => args.push(String::from("-n")), // Compare according to string numerical value
            2 => args.push(String::from("-f")), // Fold lower case to upper case characters
            3 => args.push(generate_random_string(rng.gen_range(1..=10))), // Random string (to simulate file names)
            _ => args.push(String::from("-k") + &rng.gen_range(1..=5).to_string()), // Sort via a specified field
        }
    }

    args.join(" ")
}

fn generate_random_lines(count: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut lines = Vec::new();

    for _ in 0..count {
        lines.push(generate_random_string(rng.gen_range(1..=20)));
    }

    lines.join("\n")
}

fuzz_target!(|_data: &[u8]| {
    let sort_args = generate_sort_args();
    let mut args = vec![OsString::from("sort")];
    args.extend(sort_args.split_whitespace().map(OsString::from));

    // Generate random lines to sort
    let input_lines = generate_random_lines(10);

    let rust_result = generate_and_run_uumain(&args, uumain, Some(&input_lines));

    // TODO remove once uutils sort supports localization
    env::set_var("LC_ALL", "C");
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
        "sort",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
