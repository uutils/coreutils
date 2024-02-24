// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_printf::uumain;

use rand::seq::SliceRandom;
use rand::Rng;
use std::env;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::CommandResult;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};

static CMD_PATH: &str = "printf";

fn generate_escape_sequence(rng: &mut impl Rng) -> String {
    let escape_sequences = [
        "\\\"",
        "\\\\",
        "\\a",
        "\\b",
        "\\c",
        "\\e",
        "\\f",
        "\\n",
        "\\r",
        "\\t",
        "\\v",
        "\\000",
        "\\x00",
        "\\u0000",
        "\\U00000000",
        "%%",
    ];
    escape_sequences.choose(rng).unwrap().to_string()
}

fn generate_printf() -> String {
    let mut rng = rand::thread_rng();
    let format_specifiers = ["%s", "%d", "%f", "%x", "%o", "%c", "%b", "%q"];
    let mut printf_str = String::new();
    // Add a 20% chance of generating an invalid format specifier
    if rng.gen_bool(0.2) {
        printf_str.push_str("%z"); // Invalid format specifier
    } else {
        let specifier = *format_specifiers.choose(&mut rng).unwrap();
        printf_str.push_str(specifier);

        // Add a 20% chance of introducing complex format strings
        if rng.gen_bool(0.2) {
            printf_str.push_str(&format!(" %{}", rng.gen_range(1..=1000)));
        } else {
            // Add a random string or number after the specifier
            if specifier == "%s" {
                printf_str.push_str(&format!(
                    " {}",
                    generate_random_string(rng.gen_range(1..=10))
                ));
            } else {
                printf_str.push_str(&format!(" {}", rng.gen_range(1..=1000)));
            }
        }
    }

    // Add a 10% chance of including an escape sequence
    if rng.gen_bool(0.1) {
        printf_str.push_str(&generate_escape_sequence(&mut rng));
    }
    printf_str
}

fuzz_target!(|_data: &[u8]| {
    let printf_input = generate_printf();
    let mut args = vec![OsString::from("printf")];
    args.extend(printf_input.split_whitespace().map(OsString::from));
    let rust_result = generate_and_run_uumain(&args, uumain, None);

    // TODO remove once uutils printf supports localization
    env::set_var("LC_ALL", "C");
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
        "printf",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
