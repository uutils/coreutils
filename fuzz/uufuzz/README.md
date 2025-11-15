# uufuzz

A Rust library for **differential fuzzing** of command-line utilities. Originally designed for testing uutils coreutils against GNU coreutils, but can be used to compare any two implementations of command-line tools.

Differential fuzzing is a testing technique that compares the behavior of two implementations of the same functionality using randomly generated inputs. This helps identify bugs, inconsistencies, and security vulnerabilities by finding cases where implementations diverge unexpectedly.

## Features

- **Command Execution**: Run and capture output from both Rust and reference implementations
- **Result Comparison**: Detailed comparison of stdout, stderr, and exit codes with diff output
- **Input Generation**: Utilities for generating random strings, files, and test inputs
- **GNU Compatibility**: Built-in support for detecting and running GNU coreutils
- **Pretty Output**: Colorized and formatted test result display

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
uufuzz = "0.1.0"
```

### Basic Example

```rust
use std::ffi::OsString;
use uufuzz::{generate_and_run_uumain, run_gnu_cmd, compare_result};

// Your utility's main function
fn my_echo_main(args: std::vec::IntoIter<OsString>) -> i32 {
    // Implementation here
    0
}

// Test against GNU implementation
let args = vec![OsString::from("echo"), OsString::from("hello")];

// Run your implementation
let rust_result = generate_and_run_uumain(&args, my_echo_main, None);

// Run GNU implementation
let gnu_result = run_gnu_cmd("echo", &args[1..], false, None).unwrap();

// Compare results
compare_result("echo", "hello", None, &rust_result, &gnu_result, true);
```

### With Pipe Input

```rust
let pipe_input = "test data";
let rust_result = generate_and_run_uumain(&args, my_cat_main, Some(pipe_input));
let gnu_result = run_gnu_cmd("cat", &args[1..], false, Some(pipe_input)).unwrap();
compare_result("cat", "", Some(pipe_input), &rust_result, &gnu_result, true);
```

### Random Input Generation

```rust
use uufuzz::{generate_random_string, generate_random_file};

// Generate random string up to 50 characters
let random_input = generate_random_string(50);

// Generate random temporary file
let file_path = generate_random_file().expect("Failed to create file");
```

## Use Cases

### Fuzzing Testing
Perfect for libFuzzer-based differential fuzzing:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use uufuzz::*;

fuzz_target!(|_data: &[u8]| {
    let args = generate_test_args();
    let rust_result = generate_and_run_uumain(&args, my_utility_main, None);
    let gnu_result = run_gnu_cmd("utility", &args[1..], false, None).unwrap();
    compare_result("utility", &format!("{:?}", args), None, &rust_result, &gnu_result, true);
});
```

### Integration Testing
Use in regular test suites to verify compatibility:

```rust
#[test]
fn test_basic_functionality() {
    let args = vec![OsString::from("sort"), OsString::from("-n")];
    let input = "3\n1\n2\n";

    let rust_result = generate_and_run_uumain(&args, sort_main, Some(input));
    let gnu_result = run_gnu_cmd("sort", &args[1..], false, Some(input)).unwrap();

    assert_eq!(rust_result.stdout, gnu_result.stdout);
    assert_eq!(rust_result.exit_code, gnu_result.exit_code);
}
```

## Environment Variables

- `LC_ALL=C` - Automatically set when running GNU commands for consistent behavior

## Platform Support

- **Linux**: Full support with GNU coreutils
- **macOS**: Works with GNU coreutils via Homebrew (`brew install coreutils`)
- **Windows**: Limited support (depends on available reference implementations)

## Examples

The library includes several working examples in the `examples/` directory:

### Running Examples

```bash
# Basic differential comparison
cargo run --example basic_echo

# Pipe input handling
cargo run --example pipe_input

# Simple integration testing (recommended approach)
cargo run --example simple_integration

# Complex integration testing (demonstrates file descriptor handling issues)
cargo run --example integration_testing
```

## License

Licensed under the MIT License, same as uutils coreutils.
