// This file is part of the uutils coreutils package.
// Round-trip tests for stty save/restore using the colon-separated hex format

use std::path::Path;
use uutests::new_ucmd;

fn dev_tty_available() -> bool {
    #[cfg(unix)]
    {
        Path::new("/dev/tty").exists()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[test]
fn round_trip_save_restore_dev_tty() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping round-trip test");
        return;
    }

    // Capture current settings in stty-readable form
    let res = new_ucmd!().args(&["-F", "/dev/tty", "-g"]).succeeds();
    let save = res.stdout_str().trim().to_string();
    assert!(!save.is_empty());

    // Apply the saved string back
    new_ucmd!().args(&["-F", "/dev/tty", &save]).succeeds();

    // Verify the state is preserved by comparing with a new -g
    let res2 = new_ucmd!().args(&["-F", "/dev/tty", "-g"]).succeeds();
    let save2 = res2.stdout_str().trim().to_string();
    assert_eq!(save, save2, "stty -g did not round-trip to the same state");
}

#[test]
fn malformed_save_strings_dev_tty() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping malformed tests");
        return;
    }

    // Start from a valid save string
    let res = new_ucmd!().args(&["-F", "/dev/tty", "-g"]).succeeds();
    let save = res.stdout_str().trim().to_string();

    // Case 1: remove the last field (CC count mismatch)
    if let Some((prefix, _)) = save.rsplit_once(':') {
        let truncated = prefix.to_string();
        new_ucmd!()
            .args(&["-F", "/dev/tty", &truncated])
            .fails_with_code(1)
            .stderr_contains("invalid argument");
    }

    // Case 2: non-hex content in a CC position
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    if parts.len() >= 5 {
        parts[4] = "zz".into(); // corrupt first CC
        let corrupted = parts.join(":");
        new_ucmd!()
            .args(&["-F", "/dev/tty", &corrupted])
            .fails_with_code(1)
            .stderr_contains("invalid");
    }
}
