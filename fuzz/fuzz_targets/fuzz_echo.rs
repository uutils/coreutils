#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_echo::uumain; // Changed from uu_printf to uu_echo

use rand::prelude::SliceRandom;
use rand::Rng;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};

static CMD_PATH: &str = "/usr/bin/echo"; // Changed from "printf" to "echo"

fn generate_echo() -> String {
    let mut rng = rand::thread_rng();
    let mut echo_str = String::new();

    // Randomly decide whether to include options
    let include_n = rng.gen_bool(0.1); // 10% chance
    let include_e = rng.gen_bool(0.1); // 10% chance
    let include_E = rng.gen_bool(0.1); // 10% chance
                                       // --help and --version are typically not included in fuzzing as they don't change output format

    if include_n {
        echo_str.push_str("-n ");
    }
    if include_e {
        echo_str.push_str("-e ");
    }
    if include_E {
        echo_str.push_str("-E ");
    }

    // Add a random string
    echo_str.push_str(&generate_random_string(rng.gen_range(1..=10)));

    // Include escape sequences if -e is enabled
    if include_e {
        // Add a 10% chance of including an escape sequence
        if rng.gen_bool(0.1) {
            echo_str.push_str(&generate_escape_sequence(&mut rng)); // This function should handle echo-specific sequences
        }
    }

    echo_str
}

// You should also modify the generate_escape_sequence function to include echo-specific sequences
fn generate_escape_sequence(rng: &mut impl Rng) -> String {
    let escape_sequences = [
        "\\\\", "\\a", "\\b", "\\c", "\\e", "\\f", "\\n", "\\r", "\\t", "\\v",
        "\\0NNN", // You can randomly generate NNN
        "\\xHH",  // You can randomly generate HH
                  // ... other sequences
    ];
    escape_sequences.choose(rng).unwrap().to_string()
}

fuzz_target!(|_data: &[u8]| {
    let echo_input = generate_echo(); // Changed from generate_printf to generate_echo
    let mut args = vec![OsString::from("echo")]; // Changed from "printf" to "echo"
    args.extend(echo_input.split_whitespace().map(OsString::from));
    let rust_result = generate_and_run_uumain(&args, uumain); // uumain function from uu_echo

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
        "echo",
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
