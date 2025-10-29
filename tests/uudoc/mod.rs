// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{env, path::PathBuf, process::Command, sync::OnceLock};

static UUDOC_BINARY_PATH: OnceLock<PathBuf> = OnceLock::new();

fn get_uudoc_command() -> Command {
    let uudoc_binary = UUDOC_BINARY_PATH.get_or_init(|| {
        let coreutils_binary = PathBuf::from(env!("CARGO_BIN_EXE_coreutils"));
        coreutils_binary.parent().unwrap().join("uudoc")
    });
    Command::new(uudoc_binary)
}

/// Helper function to test generic man page formatting for a given utility
/// This tests only the structural formatting, not specific content
fn test_manpage_formatting_for_utility(utility: &str) {
    test_manpage_formatting_for_utility_with_lang(utility, None)
}

/// Helper function to test generic man page formatting for a given utility with specific LANG
/// This tests only the structural formatting, not specific content
fn test_manpage_formatting_for_utility_with_lang(utility: &str, lang: Option<&str>) {
    let mut binding = get_uudoc_command();
    let mut command = binding.arg("manpage").arg(utility);

    if let Some(lang_val) = lang {
        command = command.env("LANG", lang_val);
    }

    let output = command.output().unwrap_or_else(|_| {
        panic!(
            "Failed to execute manpage command for {} with lang {:?}",
            utility, lang
        )
    });

    let lang_desc = lang.unwrap_or("default");
    assert!(
        output.status.success(),
        "Command failed with status: {} for utility {} with lang {}",
        output.status,
        utility,
        lang_desc
    );

    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {} for utility {} with lang {}",
        String::from_utf8_lossy(&output.stderr),
        utility,
        lang_desc
    );

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Basic structure checks
    assert!(
        output_str.contains("\n.TH"),
        "Missing .TH header for utility {} with lang {}",
        utility,
        lang_desc
    );
    assert!(
        output_str.contains(utility),
        "Utility name '{}' not found in manpage with lang {}",
        utility,
        lang_desc
    );

    // Test formatting - validate both section headers and bullet points
    // We only care about the structural formatting, not the content
    let lines: Vec<&str> = output_str.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        // Check that section headers (lines ending with ':') are followed by empty lines
        if line.trim().ends_with(':') && i + 1 < lines.len() {
            let next_line = lines[i + 1];
            let properly_formatted = next_line.trim().is_empty();

            assert!(
                properly_formatted,
                "Section header formatting issue in {} manpage: line {} '{}' is not followed by empty line",
                utility,
                i + 1,
                line
            );
        }

        // Check that bullet points are properly preceded by empty lines
        if line.trim_start().starts_with("\\- ") {
            let properly_formatted = if i == 0 {
                // First line can't be a bullet in a proper man page
                false
            } else {
                let prev_line = lines[i - 1];
                // Should be preceded by an empty line (after trimming whitespace)
                prev_line.trim().is_empty()
            };

            assert!(
                properly_formatted,
                "Bullet point formatting issue in {} manpage: line {} '{}' is not properly preceded by empty line",
                utility,
                i + 1,
                line
            );
        }
    }
}

#[test]
fn test_manpage_generation() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("test")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("\n.TH"), "{output_str}");
    assert!(output_str.contains('1'), "{output_str}");
}

#[test]
fn test_manpage_coreutils() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("coreutils")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("\n.TH"), "{output_str}");
    assert!(output_str.contains("coreutils"), "{output_str}");
}

#[test]
fn test_manpage_invalid_utility() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("nonexistent_utility")
        .output()
        .expect("Failed to execute command");

    // Should fail for invalid utility
    assert!(!output.status.success(), "Command should have failed");
}

#[test]
fn test_completion_generation() {
    let output = get_uudoc_command()
        .arg("completion")
        .arg("ls")
        .arg("powershell")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("using namespace System.Management.Automation"),
        "{output_str}"
    );
}

// Prevent regression to:
//
// ❯ uudoc manpage base64 | rg --fixed-strings -- 'base32'
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
#[test]
fn test_manpage_base64() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("base64")
        .env("LANG", "C") // force locale to english to avoid issues with manpage output
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("base64 alphabet"));
    assert!(!output_str.to_ascii_lowercase().contains("base32"));
}

#[test]
fn test_manpage_formatting_all_utilities() {
    // Test man page formatting for a comprehensive set of utilities to ensure
    // the formatting issue is consistent across all utilities

    // Test a representative sample of utilities
    let utilities_to_test = vec!["cat", "test", "wc", "uniq", "echo", "head", "tail", "cut"];

    for utility in utilities_to_test {
        test_manpage_formatting_for_utility(utility);
    }
}

#[test]
fn test_manpage_formatting_french() {
    // Test generic formatting for the test utility with French locale
    // This only checks structural formatting, not specific French content
    test_manpage_formatting_for_utility_with_lang("test", Some("fr_FR.UTF-8"));
}
