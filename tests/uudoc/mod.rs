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
        String::from_utf8_lossy(&output.stderr).contains("Warning: No tldr archive found"),
        "stderr should contains tldr alert",
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("\n.TH ls"), "{output_str}");
    assert!(output_str.contains('1'), "{output_str}");
    assert!(output_str.contains("\n.SH NAME\nls"), "{output_str}");
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
        String::from_utf8_lossy(&output.stderr).contains("Warning: No tldr archive found"),
        "stderr should contains tldr alert",
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("\n.TH coreutils"), "{output_str}");
    assert!(output_str.contains("coreutils"), "{output_str}");
    assert!(output_str.contains("\n.SH NAME\ncoreutils"), "{output_str}");
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
        String::from_utf8_lossy(&output.stderr).contains("Warning: No tldr archive found"),
        "stderr should contains tldr alert",
    );

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("base64 alphabet"));
    assert!(!output_str.to_ascii_lowercase().contains("base32"));
}
