//! Tests on the `uudoc` binary.
//!
//! To run the uudoc
//! ```
//! cargo run --bin uudoc --features uudoc
//! ```
//!
//! To run the tests
//! ```
//! cargo test --features uudoc
//! ```

use std::env;
pub const TESTS_BINARY: &str = env!("CARGO_BIN_EXE_uudoc");

// Set the environment variable for any tests

// Use the ctor attribute to run this function before any tests
#[ctor::ctor]
fn init() {
    // No need for unsafe here
    unsafe {
        std::env::set_var("UUTESTS_BINARY_PATH", TESTS_BINARY);
    }
    // Print for debugging
    eprintln!("Setting UUTESTS_BINARY_PATH={TESTS_BINARY}");
}

/// Run the `uudoc` command and return the output as a vector of strings.
/// # Errors
/// If the command fails to execute or if the output cannot be converted to UTF-8, an `io::Error` is returned.
fn run_write_doc() -> Vec<String> {
    use std::process::Command;
    use uutests::util::TestScenario;

    let scenario = TestScenario::new("");
    let output = Command::new(&scenario.bin_path).output().unwrap();
    assert!(
        output.status.success(),
        "uudoc command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .filter(|line| line.starts_with("Wrote"))
        .collect::<Vec<String>>()
}

#[test]
fn uudoc_check_test() {
    let pages = run_write_doc();
    println!("Pages written: {pages:?}\n");
    // assert wrote to the correct file
    let path_test = pages.iter().find(|line| line.contains("test.md")).unwrap();
    let correct_path_test = path_test
        .strip_prefix("Wrote to '")
        .unwrap()
        .strip_suffix("'")
        .unwrap()
        .to_string();
    assert_eq!(correct_path_test, "docs/src/utils/test.md");

    // open the file
    let content = std::fs::read_to_string(correct_path_test).unwrap();
    assert!(content.contains(
        "```
test EXPRESSION
test
[ EXPRESSION ]
[ ]
[ OPTION
```
"
    ));
}

#[test]
fn uudoc_check_sums() {
    let pages = run_write_doc();
    let sums = [
        "md5sum",
        "sha1sum",
        "sha224sum",
        "sha256sum",
        "sha384sum",
        "sha512sum",
        "sha3sum",
        "sha3-224sum",
        "sha3-256sum",
        "sha3-384sum",
        "sha3-512sum",
        "shake128sum",
        "shake256sum",
        "b2sum",
        "b3sum",
    ];
    for one_sum in sums {
        let path = pages.iter().find(|line| line.contains(one_sum)).unwrap();
        let correct_path = path
            .strip_prefix("Wrote to '")
            .unwrap()
            .strip_suffix("'")
            .unwrap()
            .to_string();
        assert!(correct_path.contains("docs/src/utils/"));
        assert!(correct_path.contains(one_sum));
        // open the file
        let content = std::fs::read_to_string(correct_path).unwrap();
        let formatted = format!("```\n{} [OPTIONS]... [FILE]...\n```", one_sum);
        assert!(content.contains(&formatted));
    }
}
