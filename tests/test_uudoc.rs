// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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

#![cfg(feature = "uudoc")]

use std::env;
pub const TESTS_BINARY: &str = env!("CARGO_BIN_EXE_uudoc");

// Set the environment variable for any tests

// Use the ctor attribute to run this function before any tests
#[ctor::ctor]
fn init() {
    // No need for unsafe here
    unsafe {
        env::set_var("UUTESTS_BINARY_PATH", TESTS_BINARY);
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

fn get_doc_file_from_output(output: &str) -> (String, String) {
    let correct_path_test = output
        .strip_prefix("Wrote to '")
        .unwrap()
        .strip_suffix("'")
        .unwrap()
        .to_string();
    let content = std::fs::read_to_string(&correct_path_test);
    let content = match content {
        Ok(content) => content,
        Err(e) => {
            panic!(
                "Failed to read file {correct_path_test}: {e} from {:?}",
                env::current_dir(),
            );
        }
    };
    (correct_path_test, content)
}

#[test]
fn uudoc_check_test() {
    let pages = run_write_doc();
    // assert wrote to the correct file
    let path_test = pages.iter().find(|line| line.contains("test.md")).unwrap();
    let (correct_path, content) = get_doc_file_from_output(path_test);

    // open the file
    assert!(
        content.contains(
            "```
test EXPRESSION
test
[ EXPRESSION ]
[ ]
[ OPTION
```
",
        ),
        "{correct_path} does not contains the required text"
    );
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
        "b2sum",
    ];
    for one_sum in sums {
        let output_path = pages
            .iter()
            .find(|one_line| one_line.contains(one_sum))
            .unwrap();
        let (correct_path, content) = get_doc_file_from_output(output_path);
        let formatted = format!("```\n{one_sum} [OPTIONS] [FILE]...\n```");
        assert!(
            content.contains(&formatted),
            "Content of {correct_path} does not contain the expected format: {formatted}",
        );
    }
}
