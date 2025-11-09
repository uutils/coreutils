// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

use std::collections::HashSet;
use std::env;
use std::process::Command;
use std::str;

// Use the same binary path as other tests
pub const TESTS_BINARY: &str = env!("CARGO_BIN_EXE_coreutils");

/// Get list of all enabled utilities from the build-time generated map.
/// Uses `include_str!` to read the generated `uutils_map.rs` at compile time,
/// avoiding runtime execution while staying in sync with the actual build.
fn get_all_enabled_utilities() -> Vec<String> {
    // Read the generated utility map file at compile time
    const UUTILS_MAP: &str = include_str!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

    // Extract utility names from lines like: ("arch", (arch::uumain, arch::uu_app)),
    UUTILS_MAP
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("(\"") && line.contains(", (") {
                let end_quote = line[2..].find('"')?;
                Some(line[2..2 + end_quote].to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Utilities that should be skipped in tests due to special behavior
fn get_utilities_to_skip() -> HashSet<&'static str> {
    let mut skip_set = HashSet::new();

    // Utilities that don't follow standard help patterns
    skip_set.insert("false"); // Always exits with 1
    skip_set.insert("true"); // Always exits with 0, no help
    skip_set.insert("["); // Special test utility syntax
    skip_set.insert("test"); // By design, doesn't show --help (use [ --help instead)

    // Alias for ls
    skip_set.insert("dir");
    skip_set.insert("vdir");

    // Utilities that don't show standard clap error messages by design
    skip_set.insert("echo"); // Prints arguments as-is, doesn't use clap for validation
    skip_set.insert("printf"); // Uses custom argument parsing, doesn't show clap errors
    skip_set.insert("expr"); // Uses custom argument parsing, doesn't show clap errors

    // Utilities with special error handling that work but don't follow standard patterns
    let utilities_with_special_handling = [
        "seq",  // Custom numeric validation with localized messages
        "tail", // Complex file following logic
        "stty", // Terminal-specific error handling
    ];

    for utility in &utilities_with_special_handling {
        skip_set.insert(utility);
    }

    skip_set
}

/// Helper function to create a Command for a utility.
/// Uses the multicall binary (`TESTS_BINARY`) and passes the utility name as an argument.
fn create_utility_command(utility_name: &str) -> Command {
    let uu_name = format!("uu_{utility_name}");
    let canonical_name = uucore::get_canonical_util_name(&uu_name);
    let mut cmd = Command::new(TESTS_BINARY);
    cmd.arg(canonical_name);
    cmd
}

/// Test that help messages contain color codes when `CLICOLOR_FORCE=1`
#[test]
fn test_help_messages_have_colors() {
    let utilities = get_all_enabled_utilities();
    let skip_utilities = get_utilities_to_skip();

    for utility in &utilities {
        if skip_utilities.contains(utility.as_str()) {
            continue;
        }
        println!("Testing colors for {utility}");

        let output = create_utility_command(utility)
            .arg("--help")
            .env("CLICOLOR_FORCE", "1")
            .env("LANG", "en_US.UTF-8")
            .output();

        match output {
            Ok(result) => {
                let stdout = str::from_utf8(&result.stdout).unwrap_or("");

                // Check for ANSI color codes in help output
                // We expect to see bold+underline codes for headers like "Usage:"
                let has_colors = stdout.contains("\x1b[1m\x1b[4m") && stdout.contains("\x1b[0m");

                if !has_colors {
                    println!("Help output for {utility}:\n{stdout}");
                }

                assert!(
                    has_colors,
                    "Utility '{utility}' help message should contain ANSI color codes for headers"
                );
            }
            Err(e) => {
                panic!("Failed to execute {utility} --help: {e}");
            }
        }
    }
}

/// Test that error messages contain color codes when `CLICOLOR_FORCE=1`
#[test]
fn test_error_messages_have_colors() {
    let utilities = get_all_enabled_utilities();
    let skip_utilities = get_utilities_to_skip();

    for utility in &utilities {
        if skip_utilities.contains(utility.as_str()) {
            continue;
        }
        println!("Testing error colors for {utility}");

        let mut cmd = create_utility_command(utility);
        let uu_name = format!("uu_{utility}");
        let binary_name = uucore::get_canonical_util_name(&uu_name);

        // For hashsum aliases, we need to pass the hash algorithm as a subcommand
        if binary_name == "hashsum" && utility != "hashsum" {
            // Extract the hash algorithm from the utility name
            let algo = utility.trim_end_matches("sum");
            cmd.arg(algo);
        }

        let output = cmd
            .arg("--invalid-option-that-should-not-exist")
            .env("CLICOLOR_FORCE", "1")
            .env("LANG", "en_US.UTF-8")
            .output();

        match output {
            Ok(result) => {
                let stderr = str::from_utf8(&result.stderr).unwrap_or("");

                // Check for red error text and yellow invalid option
                let has_red_error = stderr.contains("\x1b[31merror") && stderr.contains("\x1b[0m");
                let has_yellow_option =
                    stderr.contains("\x1b[33m--invalid-option-that-should-not-exist\x1b[0m");

                if !has_red_error || !has_yellow_option {
                    println!("Error output for {utility}:\n{stderr}");
                }

                assert!(
                    has_red_error,
                    "Utility '{utility}' should show red colored 'error' in error messages"
                );
                assert!(
                    has_yellow_option,
                    "Utility '{utility}' should show yellow colored invalid options in error messages"
                );
            }
            Err(e) => {
                panic!("Failed to execute {utility} with invalid option: {e}");
            }
        }
    }
}

/// Test that help messages are translated to French
#[test]
fn test_help_messages_french_translation() {
    let utilities = get_all_enabled_utilities();
    let skip_utilities = get_utilities_to_skip();

    for utility in &utilities {
        if skip_utilities.contains(utility.as_str()) {
            continue;
        }
        println!("Testing French translation for {utility}");

        let output = create_utility_command(utility)
            .arg("--help")
            .env("CLICOLOR_FORCE", "1")
            .env("LANG", "fr_FR.UTF-8")
            .env("LC_ALL", "fr_FR.UTF-8")
            .output();

        match output {
            Ok(result) => {
                let stdout = str::from_utf8(&result.stdout).unwrap_or("");

                // Check for French translation of "Usage:" -> "Utilisation:"
                let has_french_usage = stdout.contains("Utilisation:");

                if !has_french_usage {
                    println!("Help output for {utility} (French):\n{stdout}");
                }

                assert!(
                    has_french_usage,
                    "Utility '{utility}' help message should be translated to French (contain 'Utilisation:')"
                );
            }
            Err(e) => {
                panic!("Failed to execute {utility} --help in French: {e}");
            }
        }
    }
}

/// Test that error messages are translated to French
#[test]
fn test_error_messages_french_translation() {
    let utilities = get_all_enabled_utilities();
    let skip_utilities = get_utilities_to_skip();

    for utility in &utilities {
        if skip_utilities.contains(utility.as_str()) {
            continue;
        }
        println!("Testing French error translation for {utility}");

        let mut cmd = create_utility_command(utility);
        let uu_name = format!("uu_{utility}");
        let binary_name = uucore::get_canonical_util_name(&uu_name);

        // For hashsum aliases, we need to pass the hash algorithm as a subcommand
        if binary_name == "hashsum" && utility != "hashsum" {
            // Extract the hash algorithm from the utility name
            let algo = utility.trim_end_matches("sum");
            cmd.arg(algo);
        }

        let output = cmd
            .arg("--invalid-option-that-should-not-exist")
            .env("CLICOLOR_FORCE", "1")
            .env("LANG", "fr_FR.UTF-8")
            .env("LC_ALL", "fr_FR.UTF-8")
            .output();

        match output {
            Ok(result) => {
                let stderr = str::from_utf8(&result.stderr).unwrap_or("");

                // Check for French translation of "error" -> "erreur"
                let has_french_error = stderr.contains("erreur");

                if !has_french_error {
                    println!("Error output for {utility} (French):\n{stderr}");
                }

                assert!(
                    has_french_error,
                    "Utility '{utility}' error message should be translated to French (contain 'erreur')"
                );
            }
            Err(e) => {
                panic!("Failed to execute {utility} with invalid option in French: {e}");
            }
        }
    }
}

/// Test that colors work with French translations
#[test]
fn test_french_colored_error_messages() {
    let utilities = get_all_enabled_utilities();
    let skip_utilities = get_utilities_to_skip();

    for utility in &utilities {
        if skip_utilities.contains(utility.as_str()) {
            continue;
        }
        println!("Testing French colored errors for {utility}");

        let mut cmd = create_utility_command(utility);
        let uu_name = format!("uu_{utility}");
        let binary_name = uucore::get_canonical_util_name(&uu_name);

        // For hashsum aliases, we need to pass the hash algorithm as a subcommand
        if binary_name == "hashsum" && utility != "hashsum" {
            // Extract the hash algorithm from the utility name
            let algo = utility.trim_end_matches("sum");
            cmd.arg(algo);
        }

        let output = cmd
            .arg("--invalid-option-that-should-not-exist")
            .env("CLICOLOR_FORCE", "1")
            .env("LANG", "fr_FR.UTF-8")
            .env("LC_ALL", "fr_FR.UTF-8")
            .output();

        match output {
            Ok(result) => {
                let stderr = str::from_utf8(&result.stderr).unwrap_or("");

                // Check for red colored French error text and yellow invalid option
                let has_red_erreur =
                    stderr.contains("\x1b[31merreur") && stderr.contains("\x1b[0m");
                let has_yellow_option =
                    stderr.contains("\x1b[33m--invalid-option-that-should-not-exist\x1b[0m");

                if !has_red_erreur || !has_yellow_option {
                    println!("French error output for {utility}:\n{stderr}");
                }

                assert!(
                    has_red_erreur,
                    "Utility '{utility}' should show red colored 'erreur' in French error messages"
                );
                assert!(
                    has_yellow_option,
                    "Utility '{utility}' should show yellow colored invalid options in French error messages"
                );
            }
            Err(e) => {
                panic!("Failed to execute {utility} with invalid option in French: {e}");
            }
        }
    }
}
