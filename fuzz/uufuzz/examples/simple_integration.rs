// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use uufuzz::{CommandResult, run_gnu_cmd};

fn main() {
    println!("=== Simple Integration Testing uufuzz Example ===");
    println!("This demonstrates how to use uufuzz to compare against GNU tools");
    println!("without the complex file descriptor manipulation.\n");

    // Test cases that work well with external command comparison
    let test_cases = [
        (
            "echo test",
            "echo",
            vec![OsString::from("hello"), OsString::from("world")],
            None,
        ),
        (
            "echo with flag",
            "echo",
            vec![OsString::from("-n"), OsString::from("no-newline")],
            None,
        ),
        (
            "cat with input",
            "cat",
            vec![],
            Some("Hello from cat!\nLine 2\n"),
        ),
        ("sort basic", "sort", vec![], Some("zebra\napple\nbanana\n")),
        (
            "sort numeric",
            "sort",
            vec![OsString::from("-n")],
            Some("10\n2\n1\n20\n"),
        ),
    ];

    for (test_name, cmd, args, input) in test_cases {
        println!("--- {} ---", test_name);

        // Run GNU command
        match run_gnu_cmd(cmd, &args, false, input) {
            Ok(gnu_result) => {
                println!("✓ GNU {} succeeded", cmd);
                println!(
                    "  Stdout: {:?}",
                    gnu_result.stdout.trim().replace('\n', "\\n")
                );
                println!("  Exit code: {}", gnu_result.exit_code);

                // This demonstrates how you would compare results
                // In real usage, you'd run your implementation and compare:
                // let my_result = run_my_implementation(&args, input);
                // assert_eq!(my_result.stdout, gnu_result.stdout);
                // assert_eq!(my_result.exit_code, gnu_result.exit_code);
            }
            Err(error_result) => {
                println!(
                    "⚠ GNU {} failed or not available: {}",
                    cmd, error_result.stderr
                );
                println!("  This is normal if GNU coreutils isn't installed");
            }
        }
        println!();
    }

    println!("=== Practical Example: Compare two echo implementations ===");

    // Simple echo comparison
    let args = vec![OsString::from("hello"), OsString::from("world")];
    match run_gnu_cmd("echo", &args, false, None) {
        Ok(gnu_result) => {
            println!("GNU echo result: {:?}", gnu_result.stdout.trim());

            // Simulate our own echo implementation result
            let our_result = CommandResult {
                stdout: "hello world\n".to_string(),
                stderr: String::new(),
                exit_code: 0,
            };

            if our_result.stdout.trim() == gnu_result.stdout.trim()
                && our_result.exit_code == gnu_result.exit_code
            {
                println!("✓ Our echo matches GNU echo!");
            } else {
                println!("✗ Our echo differs from GNU echo");
                println!("  Our result: {:?}", our_result.stdout.trim());
                println!("  GNU result: {:?}", gnu_result.stdout.trim());
            }
        }
        Err(_) => {
            println!("Cannot compare - GNU echo not available");
        }
    }

    println!("\n=== Example completed ===");
    println!("This approach is simpler and more reliable for integration testing.");
}
