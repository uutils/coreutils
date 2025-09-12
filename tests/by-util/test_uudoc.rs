// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[test]
#[cfg(feature = "true")]
fn test_manpage_generation() {
    use std::process::{Command, Stdio};

    // Check if uudoc binary exists
    let uudoc_path = if std::path::Path::new("target/debug/uudoc").exists() {
        "target/debug/uudoc"
    } else if std::path::Path::new("target/release/uudoc").exists() {
        "target/release/uudoc"
    } else {
        println!("Skipping test: uudoc binary not found. Build with --features uudoc first.");
        return;
    };

    let child = Command::new(uudoc_path)
        .arg("manpage")
        .arg("true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("\n.TH"), "{output_str:?}");
    assert!(output_str.contains('1'), "{output_str:?}");
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
    use std::process::{Command, Stdio};
    unsafe {
        // force locale to english to avoid issues with manpage output
        std::env::set_var("LANG", "C");
    }

    // Check if uudoc binary exists
    let uudoc_path = if std::path::Path::new("target/debug/uudoc").exists() {
        "target/debug/uudoc"
    } else if std::path::Path::new("target/release/uudoc").exists() {
        "target/release/uudoc"
    } else {
        println!("Skipping test: uudoc binary not found. Build with --features uudoc first.");
        return;
    };

    let child = Command::new(uudoc_path)
        .arg("manpage")
        .arg("base64")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code().unwrap(), 0);

    let stdout_str = std::str::from_utf8(&output.stdout).unwrap();
    assert!(stdout_str.contains("base64 alphabet"));
    assert!(!stdout_str.to_ascii_lowercase().contains("base32"));
}

#[test]
fn test_manpage_coreutils() {
    use std::process::{Command, Stdio};

    // Check if uudoc binary exists
    let uudoc_path = if std::path::Path::new("target/debug/uudoc").exists() {
        "target/debug/uudoc"
    } else if std::path::Path::new("target/release/uudoc").exists() {
        "target/release/uudoc"
    } else {
        println!("Skipping test: uudoc binary not found. Build with --features uudoc first.");
        return;
    };

    let child = Command::new(uudoc_path)
        .arg("manpage")
        .arg("coreutils")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("\n.TH"), "{output_str:?}");
    assert!(output_str.contains("coreutils"), "{output_str:?}");
}

#[test]
fn test_manpage_invalid_utility() {
    use std::process::{Command, Stdio};

    // Check if uudoc binary exists
    let uudoc_path = if std::path::Path::new("target/debug/uudoc").exists() {
        "target/debug/uudoc"
    } else if std::path::Path::new("target/release/uudoc").exists() {
        "target/release/uudoc"
    } else {
        println!("Skipping test: uudoc binary not found. Build with --features uudoc first.");
        return;
    };

    let child = Command::new(uudoc_path)
        .arg("manpage")
        .arg("nonexistent_utility")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = child.wait_with_output().unwrap();
    // Should fail for invalid utility
    assert_ne!(output.status.code(), Some(0));
}
