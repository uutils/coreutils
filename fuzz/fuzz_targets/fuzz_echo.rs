#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_echo::uumain;

use rand::prelude::SliceRandom;
use rand::Rng;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};

static CMD_PATH: &str = "echo";

fn generate_echo() -> String {
    let mut rng = rand::thread_rng();
    let mut echo_str = String::new();

    // Randomly decide whether to include options
    let include_n = rng.gen_bool(0.1); // 10% chance
    let include_e = rng.gen_bool(0.1); // 10% chance
    #[allow(non_snake_case)]
    let include_E = rng.gen_bool(0.1); // 10% chance

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
            echo_str.push_str(&generate_escape_sequence(&mut rng));
        }
    }

    echo_str
}

fn generate_escape_sequence(rng: &mut impl Rng) -> String {
    let escape_sequences = [
        "\\\\", "\\a", "\\b", "\\c", "\\e", "\\f", "\\n", "\\r", "\\t", "\\v", "\\0NNN", "\\xHH",
    ];
    // \0NNN and \xHH need more work
    escape_sequences.choose(rng).unwrap().to_string()
}

fuzz_target!(|_data: &[u8]| {
    let echo_input = generate_echo();
    let mut args = vec![OsString::from("echo")];
    args.extend(echo_input.split_whitespace().map(OsString::from));
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
        "echo",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        true,
    );
});
