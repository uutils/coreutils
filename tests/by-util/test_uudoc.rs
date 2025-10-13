// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{env, path::PathBuf, sync::OnceLock};

use uutests::util::TestScenario;

static UUDOC_BINARY_PATH: OnceLock<PathBuf> = OnceLock::new();

fn uudoc_bin() -> &'static str {
    UUDOC_BINARY_PATH
        .get_or_init(|| {
            let coreutils_binary = PathBuf::from(env!("CARGO_BIN_EXE_coreutils"));
            coreutils_binary.parent().unwrap().join("uudoc")
        })
        .to_str()
        .unwrap()
}

fn get_uudoc_command() -> uutests::util::UCommand {
    let uudoc_bin = uudoc_bin();
    let scenario = TestScenario::new("");
    scenario.cmd(uudoc_bin)
}

#[test]
fn test_manpage_generation() {
    let result = get_uudoc_command().arg("manpage").arg("ls").run();
    result.success();
    assert!(result.stderr_str().is_empty());
    let output_str = result.stdout_str();
    assert!(output_str.contains("\n.TH"), "{output_str}");
    assert!(output_str.contains('1'), "{output_str}");
}

#[test]
fn test_manpage_coreutils() {
    let result = get_uudoc_command().arg("manpage").arg("coreutils").run();
    result.success();
    assert!(result.stderr_str().is_empty());
    let output_str = result.stdout_str();
    assert!(output_str.contains("\n.TH"), "{output_str}");
    assert!(output_str.contains("coreutils"), "{output_str}");
}

#[test]
fn test_manpage_invalid_utility() {
    let result = get_uudoc_command()
        .arg("manpage")
        .arg("nonexistent_utility")
        .run();
    // Should fail for invalid utility
    result.failure();
}

#[test]
fn test_completion_generation() {
    let result = get_uudoc_command()
        .arg("completion")
        .arg("ls")
        .arg("powershell")
        .run();
    result.success();
    assert!(result.stderr_str().is_empty());
    let output_str = result.stdout_str();
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
    unsafe {
        // force locale to english to avoid issues with manpage output
        std::env::set_var("LANG", "C");
    }

    let result = get_uudoc_command().arg("manpage").arg("base64").run();
    result.success();
    assert!(result.stderr_str().is_empty());
    let output_str = result.stdout_str();
    assert!(output_str.contains("base64 alphabet"));
    assert!(!output_str.to_ascii_lowercase().contains("base32"));
}
