use rand::Rng;
use std::ffi::OsString;
use uufuzz::{generate_and_run_uumain, generate_random_string, run_gnu_cmd};

// Mock echo implementation with some bugs for demonstration
fn mock_buggy_echo_main(args: std::vec::IntoIter<OsString>) -> i32 {
    let args: Vec<OsString> = args.collect();

    let mut should_add_newline = true;
    let mut enable_escapes = false;
    let mut start_index = 1;

    // Parse arguments (simplified)
    for arg in args.iter().skip(1) {
        let arg_str = arg.to_string_lossy();
        if arg_str == "-n" {
            should_add_newline = false;
            start_index += 1;
        } else if arg_str == "-e" {
            enable_escapes = true;
            start_index += 1;
        } else {
            break;
        }
    }

    // Print arguments
    for (i, arg) in args.iter().skip(start_index).enumerate() {
        if i > 0 {
            print!(" ");
        }
        let arg_str = arg.to_string_lossy();

        if enable_escapes {
            // Simulate a bug: incomplete escape sequence handling
            let processed = arg_str.replace("\\n", "\n").replace("\\t", "\t");
            print!("{}", processed);
        } else {
            print!("{}", arg_str);
        }
    }

    if should_add_newline {
        println!();
    }

    0
}

// Generate test arguments for echo command
fn generate_echo_args() -> Vec<OsString> {
    let mut rng = rand::rng();
    let mut args = vec![OsString::from("echo")];

    // Randomly add flags
    if rng.random_bool(0.3) {
        // 30% chance
        args.push(OsString::from("-n"));
    }
    if rng.random_bool(0.2) {
        // 20% chance
        args.push(OsString::from("-e"));
    }

    // Add 1-3 random string arguments
    let num_args = rng.random_range(1..=3);
    for _ in 0..num_args {
        let arg = generate_random_string(rng.random_range(1..=15));
        args.push(OsString::from(arg));
    }

    args
}

fn main() {
    println!("=== Fuzzing Simulation uufuzz Example ===");
    println!("This simulates how libFuzzer would test our echo implementation");
    println!("against GNU echo with random inputs.\n");

    let num_tests = 10;
    let mut passed = 0;
    let mut failed = 0;

    for i in 1..=num_tests {
        println!("--- Fuzz Test {} ---", i);

        let args = generate_echo_args();
        println!(
            "Testing with args: {:?}",
            args.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>()
        );

        // Run our implementation
        let rust_result = generate_and_run_uumain(&args, mock_buggy_echo_main, None);

        // Run GNU implementation
        match run_gnu_cmd("echo", &args[1..], false, None) {
            Ok(gnu_result) => {
                // Check if results match
                let stdout_match = rust_result.stdout.trim() == gnu_result.stdout.trim();
                let exit_code_match = rust_result.exit_code == gnu_result.exit_code;

                if stdout_match && exit_code_match {
                    println!("✓ PASS: Implementations match");
                    passed += 1;
                } else {
                    println!("✗ FAIL: Implementations differ");
                    failed += 1;

                    // Show the difference in a controlled way (not panicking like compare_result)
                    if !stdout_match {
                        println!("  Stdout difference:");
                        println!(
                            "    Ours: '{}'",
                            rust_result.stdout.trim().replace('\n', "\\n")
                        );
                        println!(
                            "    GNU:  '{}'",
                            gnu_result.stdout.trim().replace('\n', "\\n")
                        );
                    }
                    if !exit_code_match {
                        println!(
                            "  Exit code difference: {} vs {}",
                            rust_result.exit_code, gnu_result.exit_code
                        );
                    }
                }
            }
            Err(error_result) => {
                println!("⚠ GNU echo not available: {}", error_result.stderr);
                println!("  Our result: '{}'", rust_result.stdout.trim());
                // Don't count this as pass or fail
                continue;
            }
        }

        println!();
    }

    println!("=== Fuzzing Results ===");
    println!("Total tests: {}", num_tests);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed > 0 {
        println!(
            "\n⚠ Found {} discrepancies! In real fuzzing, these would be investigated.",
            failed
        );
        println!("This demonstrates how differential fuzzing can find bugs in implementations.");
    } else {
        println!("\n✓ All tests passed! The implementations appear compatible.");
    }

    println!("\nIn a real libfuzzer setup, this would run thousands of iterations");
    println!("automatically with more sophisticated input generation.");
}
