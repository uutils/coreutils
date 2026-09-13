// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::io::{self, Read};
use uufuzz::{compare_result, generate_and_run_uumain, run_gnu_cmd};

// Mock cat implementation for demonstration
fn mock_cat_main(args: std::vec::IntoIter<OsString>) -> i32 {
    let _args: Vec<OsString> = args.collect();

    // Read from stdin and write to stdout
    let mut input = String::new();
    match io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            print!("{}", input);
            0
        }
        Err(_) => {
            eprintln!("Error reading from stdin");
            1
        }
    }
}

fn main() {
    println!("=== Pipe Input uufuzz Example ===");

    let args = vec![OsString::from("cat")];
    let pipe_input = "Hello from pipe!\nThis is line 2.\nAnd line 3.";

    println!("Running mock cat implementation with pipe input...");
    let rust_result = generate_and_run_uumain(&args, mock_cat_main, Some(pipe_input));

    println!("Running GNU cat with pipe input...");
    match run_gnu_cmd("cat", &args[1..], false, Some(pipe_input)) {
        Ok(gnu_result) => {
            println!("Comparing results...");
            compare_result(
                "cat",
                "",
                Some(pipe_input),
                &rust_result,
                &gnu_result,
                false,
            );
        }
        Err(error_result) => {
            println!("Failed to run GNU cat: {}", error_result.stderr);
            println!("This is expected if GNU coreutils is not installed");

            // Show what our implementation produced
            println!("\nOur implementation result:");
            println!("Stdout: '{}'", rust_result.stdout);
            println!("Stderr: '{}'", rust_result.stderr);
            println!("Exit code: {}", rust_result.exit_code);

            // Verify our mock implementation works
            if rust_result.stdout.trim() == pipe_input.trim() {
                println!("✓ Our mock cat implementation correctly echoed the pipe input");
            } else {
                println!("✗ Our mock cat implementation failed to echo the pipe input correctly");
            }
        }
    }
}
