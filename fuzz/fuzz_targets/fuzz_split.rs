// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parens

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_split::uumain;

use rand::Rng;
use std::ffi::OsString;

use uufuzz::{
    CommandResult, compare_result, generate_and_run_uumain, generate_random_string, run_gnu_cmd,
};
static CMD_PATH: &str = "split";

fn generate_split_args() -> String {
    let mut rng = rand::rng();
    let mut args = Vec::new();

    match rng.random_range(0..=9) {
        0 => {
            args.push(String::from("-a")); // Suffix length
            args.push(rng.random_range(1..=8).to_string());
        }
        1 => {
            args.push(String::from("--additional-suffix"));
            args.push(generate_random_string(5)); // Random suffix
        }
        2 => {
            args.push(String::from("-b")); // Bytes per output file
            args.push(rng.random_range(1..=1024).to_string() + "K");
        }
        3 => {
            args.push(String::from("-C")); // Line bytes
            args.push(rng.random_range(1..=1024).to_string());
        }
        4 => args.push(String::from("-d")), // Use numeric suffixes
        5 => args.push(String::from("-x")), // Use hex suffixes
        6 => {
            args.push(String::from("-l")); // Number of lines per output file
            args.push(rng.random_range(1..=1000).to_string());
        }
        7 => {
            args.push(String::from("--filter"));
            args.push(String::from("cat > /dev/null")); // Example filter command
        }
        8 => {
            args.push(String::from("-t")); // Separator
            args.push(String::from("\n")); // Newline as separator
        }
        9 => args.push(String::from("--verbose")), // Verbose
        _ => (),
    }

    args.join(" ")
}

// Function to generate a random string of lines
fn generate_random_lines(count: usize) -> String {
    let mut rng = rand::rng();
    let mut lines = Vec::new();

    for _ in 0..count {
        lines.push(generate_random_string(rng.random_range(1..=20)));
    }

    lines.join("\n")
}

fuzz_target!(|_data: &[u8]| {
    let split_args = generate_split_args();
    let mut args = vec![OsString::from("split")];
    args.extend(split_args.split_whitespace().map(OsString::from));

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
        "split",
        &format!("{:?}", &args[1..]),
        None,
        &rust_result,
        &gnu_result,
        false, // Set to true if you want to fail on stderr diff
    );
});
