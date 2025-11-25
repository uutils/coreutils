use std::ffi::OsString;
use uufuzz::{compare_result, generate_and_run_uumain, run_gnu_cmd};

// Mock echo implementation for demonstration
fn mock_echo_main(args: std::vec::IntoIter<OsString>) -> i32 {
    let args: Vec<OsString> = args.collect();

    // Skip the program name (first argument)
    for (i, arg) in args.iter().skip(1).enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg.to_string_lossy());
    }
    println!();
    0
}

fn main() {
    println!("=== Basic uufuzz Example ===");

    // Test against GNU implementation
    let args = vec![
        OsString::from("echo"),
        OsString::from("hello"),
        OsString::from("world"),
    ];

    println!("Running mock echo implementation...");
    let rust_result = generate_and_run_uumain(&args, mock_echo_main, None);

    println!("Running GNU echo...");
    match run_gnu_cmd("echo", &args[1..], false, None) {
        Ok(gnu_result) => {
            println!("Comparing results...");
            compare_result(
                "echo",
                "hello world",
                None,
                &rust_result,
                &gnu_result,
                false,
            );
        }
        Err(error_result) => {
            println!("Failed to run GNU echo: {}", error_result.stderr);
            println!("This is expected if GNU coreutils is not installed");

            // Show what our implementation produced
            println!("\nOur implementation result:");
            println!("Stdout: '{}'", rust_result.stdout);
            println!("Stderr: '{}'", rust_result.stderr);
            println!("Exit code: {}", rust_result.exit_code);
        }
    }
}
