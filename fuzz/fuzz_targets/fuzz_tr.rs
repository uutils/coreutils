// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![no_main]
use libfuzzer_sys::fuzz_target;
use std::ffi::OsString;
use uu_tr::uumain;

use rand::Rng;

mod fuzz_common;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd, CommandResult,
};
static CMD_PATH: &str = "tr";

fn generate_tr_args() -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut args = Vec::new();

    // Translate, squeeze, and/or delete characters
    let opts = ["-c", "-d", "-s", "-t"];
    for opt in &opts {
        if rng.gen_bool(0.25) {
            args.push(opt.to_string());
        }
    }

    // Generating STRING1 and optionally STRING2
    let string1 = generate_random_string(rng.gen_range(1..=20));
    args.push(string1);
    if rng.gen_bool(0.7) {
        // Higher chance to add STRING2 for translation
        let string2 = generate_random_string(rng.gen_range(1..=20));
        args.push(string2);
    }

    args
}

fuzz_target!(|_data: &[u8]| {
    let tr_args = generate_tr_args();
    let mut args = vec![OsString::from("tr")];
    args.extend(tr_args.iter().map(OsString::from));

    let input_chars = generate_random_string(100);

    let rust_result = generate_and_run_uumain(&args, uumain, Some(&input_chars));
    let gnu_result = match run_gnu_cmd(CMD_PATH, &args[1..], false, Some(&input_chars)) {
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
        "tr",
        &format!("{:?}", &args[1..]),
        Some(&input_chars),
        &rust_result,
        &gnu_result,
        false,
    );
});
