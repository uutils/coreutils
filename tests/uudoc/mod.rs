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

#[test]
fn test_manpage_generation() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("ls")
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
fn test_manpage_test_formatting() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("test")
        .env("LANG", "C")
        .output()
        .expect("Failed to execute uudoc manpage test");

    assert!(
        output.status.success(),
        "uudoc manpage test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Basic structure
    assert!(
        output_str.contains(".TH test 1"),
        "Missing .TH test 1 header: {}",
        output_str
    );

    // Exit description
    assert!(
        output_str.contains("Exit with the status determined by EXPRESSION."),
        "Missing description"
    );

    // Bullet on its own line
    assert!(
        output_str.contains("- -n STRING the length of STRING is nonzero"),
        "Bullet not on own line"
    );

    // Allow blank lines before subsections
    assert!(
        output_str.contains("- EXPRESSION1 -o EXPRESSION2 either EXPRESSION1 or EXPRESSION2 is true\n\nString operations:"),
        "Missing blank lines before String operations:"
    );
    assert!(
        output_str
            .contains("- STRING1 != STRING2 the strings are not equal\n\nInteger comparisons:"),
        "Missing blank lines before Integer comparisons:"
    );
    assert!(
        output_str.contains(
            "- INTEGER1 -ne INTEGER2 INTEGER1 is not equal to INTEGER2\n\nFile operations:"
        ),
        "Missing blank lines before File operations:"
    );
}

#[test]
fn test_manpage_test_formatting_french() {
    let output = get_uudoc_command()
        .arg("manpage")
        .arg("test")
        .env("LANG", "fr_FR.UTF-8") // Ensure French locale
        .output()
        .expect("Failed to execute uudoc manpage test (French)");

    assert!(
        output.status.success(),
        "uudoc manpage test failed (French): {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Basic structure
    assert!(
        output_str.contains(".TH test 1"),
        "Missing .TH test 1 header: {output_str}"
    );

    // French description
    assert!(
        output_str.contains("Quitter avec le statut déterminé par EXPRESSION."),
        "Missing French description"
    );

    // Bullet on own line (example)
    assert!(
        output_str.contains("- -n STRING la longueur de STRING est non nulle\n"),
        "French bullet not on own line"
    );

    // Blank lines before subsections
    assert!(
        output_str.contains("- EXPRESSION1 -o EXPRESSION2 EXPRESSION1 ou EXPRESSION2 est vraie\n\nOpérations sur les chaînes :"),
        "Missing blank lines before Opérations sur les chaînes :"
    );
    assert!(
        output_str.contains(
            "- STRING1 != STRING2 les chaînes ne sont pas égales\n\n\nComparaisons d'entiers :"
        ),
        "Missing blank lines before Comparaisons d'entiers :"
    );
    assert!(
        output_str.contains("- INTEGER1 -ne INTEGER2 INTEGER1 n'est pas égal à INTEGER2\n\n\nOpérations sur les fichiers :"),
        "Missing blank lines before Opérations sur les fichiers :"
    );
}
