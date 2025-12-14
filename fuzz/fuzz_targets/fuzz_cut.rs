// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_cut::uumain;

use rand::Rng;
use std::ffi::OsString;

use uufuzz::{
    CommandResult, compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};
static CMD_PATH: &str = "cut";

fn generate_cut_args() -> String {
    let mut rng = rand::rng();
    let arg_count = rng.random_range(1..=6);
    let mut args = Vec::new();

    for _ in 0..arg_count {
        if rng.random_bool(0.1) {
            args.push(generate_random_string(rng.random_range(1..=20)));
        } else {
            match rng.random_range(0..=4) {
                0 => args.push(String::from("-b") + &rng.random_range(1..=10).to_string()),
                1 => args.push(String::from("-c") + &rng.random_range(1..=10).to_string()),
                2 => args.push(String::from("-d,") + &generate_random_string(1)), // Using a comma as a default delimiter
                3 => args.push(String::from("-f") + &rng.random_range(1..=5).to_string()),
                _ => (),
            }
        }
    }

    args.join(" ")
}

fn generate_delimited_data(count: usize) -> String {
    let mut rng = rand::rng();
    let mut lines = Vec::new();

    for _ in 0..count {
        let fields = (0..rng.random_range(1..=5))
            .map(|_| generate_random_string(rng.random_range(1..=10)))
            .collect::<Vec<_>>()
            .join(",");
        lines.push(fields);
    }

    lines.join("\n")
}

fuzz_target!(|_data: &[u8]| {
    let cut_args = generate_cut_args();
    let mut args = vec![OsString::from("cut")];
    args.extend(cut_args.split_whitespace().map(OsString::from));

    let input_lines = generate_delimited_data(10);

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
        "cut",
        &format!("{:?}", &args[1..]),
        Some(&input_lines),
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
