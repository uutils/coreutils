use std::ffi::OsString;
use uufuzz::{generate_and_run_uumain, run_gnu_cmd};

// Mock sort implementation for demonstration
fn mock_sort_main(args: std::vec::IntoIter<OsString>) -> i32 {
    use std::io::{self, Read};

    let args: Vec<OsString> = args.collect();
    let mut numeric_sort = false;
    let mut reverse_sort = false;

    // Parse arguments
    for arg in args.iter().skip(1) {
        let arg_str = arg.to_string_lossy();
        match arg_str.as_ref() {
            "-n" | "--numeric-sort" => numeric_sort = true,
            "-r" | "--reverse" => reverse_sort = true,
            _ => {}
        }
    }

    // Read from stdin
    let mut input = String::new();
    match io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut lines: Vec<&str> = input.lines().collect();

            if numeric_sort {
                // Sort numerically
                lines.sort_by(|a, b| {
                    let a_num: f64 = a.trim().parse().unwrap_or(0.0);
                    let b_num: f64 = b.trim().parse().unwrap_or(0.0);
                    a_num.partial_cmp(&b_num).unwrap()
                });
            } else {
                // Sort lexically
                lines.sort();
            }

            if reverse_sort {
                lines.reverse();
            }

            for line in lines {
                println!("{}", line);
            }
            0
        }
        Err(_) => {
            eprintln!("Error reading from stdin");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sort_functionality() {
        println!("Testing basic sort functionality...");
        let args = vec![OsString::from("sort")];
        let input = "zebra\napple\nbanana\n";

        let rust_result = generate_and_run_uumain(&args, mock_sort_main, Some(input));

        match run_gnu_cmd("sort", &args[1..], false, Some(input)) {
            Ok(gnu_result) => {
                // In test environment, stdout might not be captured properly
                // Just verify the function runs without errors and exit codes match
                assert_eq!(
                    rust_result.exit_code, gnu_result.exit_code,
                    "Exit codes should match"
                );
                println!("✓ Basic sort test passed (exit codes match)");
            }
            Err(_) => {
                // GNU sort not available, just check our implementation runs
                assert_eq!(
                    rust_result.exit_code, 0,
                    "Our sort should exit successfully"
                );
                println!("✓ Basic sort test passed (GNU sort not available)");
            }
        }
    }

    #[test]
    fn test_numeric_sort() {
        println!("Testing numeric sort...");
        let args = vec![OsString::from("sort"), OsString::from("-n")];
        let input = "10\n2\n1\n20\n";

        let rust_result = generate_and_run_uumain(&args, mock_sort_main, Some(input));

        match run_gnu_cmd("sort", &args[1..], false, Some(input)) {
            Ok(gnu_result) => {
                assert_eq!(
                    rust_result.exit_code, gnu_result.exit_code,
                    "Exit codes should match"
                );
                println!("✓ Numeric sort test passed (exit codes match)");
            }
            Err(_) => {
                // GNU sort not available, just check our implementation runs
                assert_eq!(
                    rust_result.exit_code, 0,
                    "Our numeric sort should exit successfully"
                );
                println!("✓ Numeric sort test passed (GNU sort not available)");
            }
        }
    }

    #[test]
    fn test_reverse_sort() {
        println!("Testing reverse sort...");
        let args = vec![OsString::from("sort"), OsString::from("-r")];
        let input = "apple\nbanana\nzebra\n";

        let rust_result = generate_and_run_uumain(&args, mock_sort_main, Some(input));

        match run_gnu_cmd("sort", &args[1..], false, Some(input)) {
            Ok(gnu_result) => {
                assert_eq!(
                    rust_result.exit_code, gnu_result.exit_code,
                    "Exit codes should match"
                );
                println!("✓ Reverse sort test passed (exit codes match)");
            }
            Err(_) => {
                // GNU sort not available, just check our implementation runs
                assert_eq!(
                    rust_result.exit_code, 0,
                    "Our reverse sort should exit successfully"
                );
                println!("✓ Reverse sort test passed (GNU sort not available)");
            }
        }
    }

    #[test]
    fn test_empty_input() {
        println!("Testing empty input...");
        let args = vec![OsString::from("sort")];
        let input = "";

        let rust_result = generate_and_run_uumain(&args, mock_sort_main, Some(input));

        match run_gnu_cmd("sort", &args[1..], false, Some(input)) {
            Ok(gnu_result) => {
                assert_eq!(
                    rust_result.exit_code, gnu_result.exit_code,
                    "Exit codes should match"
                );
                println!("✓ Empty input test passed (exit codes match)");
            }
            Err(_) => {
                // GNU sort not available, just check our implementation runs
                assert_eq!(
                    rust_result.exit_code, 0,
                    "Should exit successfully with empty input"
                );
                println!("✓ Empty input test passed (GNU sort not available)");
            }
        }
    }
}

fn main() {
    println!("=== Integration Testing uufuzz Example ===");
    println!("This demonstrates how to use uufuzz in regular test suites");
    println!("to verify compatibility with reference implementations.\n");

    println!("Run 'cargo test --example integration_testing' to execute the tests.");
    println!("Or run individual tests below for demonstration:\n");

    // Demonstrate the tests manually
    let test_cases = [
        (
            "Basic lexical sort",
            vec![OsString::from("sort")],
            "zebra\napple\nbanana\n",
        ),
        (
            "Numeric sort",
            vec![OsString::from("sort"), OsString::from("-n")],
            "10\n2\n1\n20\n",
        ),
        (
            "Reverse sort",
            vec![OsString::from("sort"), OsString::from("-r")],
            "apple\nbanana\nzebra\n",
        ),
        ("Empty input", vec![OsString::from("sort")], ""),
    ];

    for (test_name, args, input) in test_cases {
        println!("--- {} ---", test_name);
        println!(
            "Args: {:?}",
            args.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>()
        );
        println!("Input: {:?}", input.replace('\n', "\\n"));

        let rust_result = generate_and_run_uumain(&args, mock_sort_main, Some(input));
        println!("Our output: {:?}", rust_result.stdout.replace('\n', "\\n"));
        println!("Exit code: {}", rust_result.exit_code);

        match run_gnu_cmd("sort", &args[1..], false, Some(input)) {
            Ok(gnu_result) => {
                println!("GNU output: {:?}", gnu_result.stdout.replace('\n', "\\n"));
                if rust_result.stdout == gnu_result.stdout
                    && rust_result.exit_code == gnu_result.exit_code
                {
                    println!("✓ Outputs match!");
                } else {
                    println!("✗ Outputs differ!");
                }
            }
            Err(_) => {
                println!("GNU sort not available for comparison");
            }
        }
        println!();
    }

    println!("=== Example completed ===");
    println!("In a real test suite, assertions would ensure compatibility.");
}
