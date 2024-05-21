// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore chdir

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_env::uumain;

use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::{
    compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd, CommandResult,
};
use rand::Rng;

static CMD_PATH: &str = "env";

fn generate_env_args() -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut args = Vec::new();

    let opts = ["-i", "-0", "-v", "-vv"];
    for opt in &opts {
        if rng.gen_bool(0.2) {
            args.push(opt.to_string());
        }
    }

    if rng.gen_bool(0.3) {
        args.push(format!(
            "-u={}",
            generate_random_string(rng.gen_range(3..10))
        ));
    }

    if rng.gen_bool(0.2) {
        args.push(format!("--chdir={}", "/tmp")); // Simplified example
    }

    /*
        Options not implemented for now
    if rng.gen_bool(0.15) {
        let sig_opts = ["--block-signal"];//, /*"--default-signal",*/ "--ignore-signal"];
        let chosen_sig_opt = sig_opts[rng.gen_range(0..sig_opts.len())];
        args.push(chosen_sig_opt.to_string());
        // Simplify by assuming SIGPIPE for demonstration
        if !chosen_sig_opt.ends_with("list-signal-handling") {
            args.push(String::from("SIGPIPE"));
        }
    }*/

    // Adding a few random NAME=VALUE pairs
    for _ in 0..rng.gen_range(0..3) {
        args.push(format!(
            "{}={}",
            generate_random_string(5),
            generate_random_string(5)
        ));
    }

    args
}

fuzz_target!(|_data: &[u8]| {
    let env_args = generate_env_args();
    let mut args = vec![OsString::from("env")];
    args.extend(env_args.iter().map(OsString::from));
    let input_lines = generate_random_string(10);

    let rust_result = generate_and_run_uumain(&args, uumain, Some(&input_lines));

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
        "env",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false,
    );
});
