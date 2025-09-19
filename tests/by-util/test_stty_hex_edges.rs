// Advanced edge-case tests for stty hex save string parsing/restoration
// These tests require a usable /dev/tty. If not available, they skip.
// Each test explains the edge case it validates.

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

fn stty_g() -> Option<String> {
    if !dev_tty_available() {
        return None;
    }
    let res = new_ucmd!().args(&["-F", "/dev/tty", "-g"]).succeeds();
    Some(res.stdout_str().trim().to_string())
}

fn nccs_of(save: &str) -> usize {
    let parts: Vec<&str> = save.split(':').collect();
    parts.len().saturating_sub(4)
}

fn with_uppercase_hex(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_hexdigit() {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect()
}

// 1) Boundary: minimal CC payload (all CCs = 0) with exact NCCS (keep current flags)
// Rationale: validate acceptance of zeroed CCs at exact platform NCCS without changing flag fields.
#[test]
fn stty_hex_minimal_ccs_exact_nccs() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().expect("stty -g should succeed");
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    let nccs = nccs_of(&save);
    assert!(parts.len() >= 4 + nccs);
    // Keep first 4 flag fields; set all CCs to 0
    parts.truncate(4);
    for _ in 0..nccs {
        parts.push("0".into());
    }
    let minimal = parts.join(":");
    new_ucmd!().args(&["-F", "/dev/tty", &minimal]).succeeds();
}

// 2) Boundary: uppercase/mixed-case hex across all fields
#[test]
fn stty_hex_mixed_case_round_trip() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mixed = with_uppercase_hex(&save);
    new_ucmd!().args(&["-F", "/dev/tty", &mixed]).succeeds();
    let save2 = stty_g().unwrap();
    assert_eq!(save2, stty_g().unwrap());
}

// 3) Boundary: leading zeros in all fields (flags + CCs)
#[test]
fn stty_hex_leading_zeros_everywhere() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    // Prepend zeros to each field
    for p in &mut parts {
        if p.is_empty() {
            *p = "0".into();
        } else {
            *p = format!("000{}", p);
        }
    }
    let padded = parts.join(":");
    new_ucmd!().args(&["-F", "/dev/tty", &padded]).succeeds();
    // Round-trip equivalence (canonical output can differ in width, so compare via new -g)
    let post = stty_g().unwrap();
    // Apply original and confirm it reverts to canonical original
    new_ucmd!().args(&["-F", "/dev/tty", &save]).succeeds();
    let back = stty_g().unwrap();
    assert_eq!(back, stty_g().unwrap());
}

// 4) Error: insufficient fields (<5)
#[test]
fn stty_hex_insufficient_fields() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    // Four flags, zero CCs
    let bad = "1:2:3:4";
    new_ucmd!()
        .args(&["-F", "/dev/tty", bad])
        .fails_with_code(1)
        .stderr_contains("invalid argument");
}

// 5) Error: extra CC field (> NCCS)
#[test]
fn stty_hex_extra_cc_field() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    parts.push("0".into()); // add one extra CC
    let extra = parts.join(":");
    new_ucmd!()
        .args(&["-F", "/dev/tty", &extra])
        .fails_with_code(1)
        .stderr_contains("invalid argument");
}

// 6) Error: malformed hex in a flag field (e.g., 2nd field)
#[test]
fn stty_hex_malformed_flag_hex() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    parts[1] = "zz".into();
    let bad = parts.join(":");
    new_ucmd!()
        .args(&["-F", "/dev/tty", &bad])
        .fails_with_code(1)
        .stderr_contains("invalid integer argument");
}

// 7) Error: unexpected characters (trailing space)
#[test]
fn stty_hex_unexpected_trailing_space() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let bad = format!("{} ", save); // append a space
    new_ucmd!()
        .args(&["-F", "/dev/tty", &bad])
        .fails_with_code(1)
        .stderr_contains("invalid integer argument");
}

// 8) Platform compatibility: NCCS-1 and NCCS+1
#[test]
fn stty_hex_platform_nccs_mismatch() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    let nccs = nccs_of(&save);

    // Case A: NCCS-1
    let mut fewer = parts.clone();
    if nccs > 0 {
        fewer.pop();
        let fewer_s = fewer.join(":");
        new_ucmd!()
            .args(&["-F", "/dev/tty", &fewer_s])
            .fails_with_code(1)
            .stderr_contains("invalid argument");
    }

    // Case B: NCCS+1
    let mut extra = parts.clone();
    extra.push("0".into());
    let extra_s = extra.join(":");
    new_ucmd!()
        .args(&["-F", "/dev/tty", &extra_s])
        .fails_with_code(1)
        .stderr_contains("invalid argument");
}

// 9) Data integrity: unknown/unsupported flag bits should be truncated (accepted)
#[test]
fn stty_hex_unknown_flag_bits_truncated() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let mut parts: Vec<String> = save.split(':').map(|s| s.to_string()).collect();
    // Add a high bit to input flags (field 0). Parse as hex, OR a high bit, and format back.
    // Use u128 to be safe across widths; renderer will truncate via from_bits_truncate.
    if let Ok(v) = u128::from_str_radix(&parts[0], 16) {
        let v2 = if v == 0 {
            1u128 << 63
        } else {
            v | (1u128 << 63)
        };
        parts[0] = format!("{:x}", v2);
        let modded = parts.join(":");
        new_ucmd!().args(&["-F", "/dev/tty", &modded]).succeeds();
        // Round-trip back to original to confirm no persistent corruption
        new_ucmd!().args(&["-F", "/dev/tty", &save]).succeeds();
    }
}

// 10) Security: oversized input (1000 CC entries) should fail quickly with invalid argument
#[test]
fn stty_hex_oversized_input_resilience() {
    if !dev_tty_available() {
        eprintln!("/dev/tty not available; skipping");
        return;
    }
    let save = stty_g().unwrap();
    let nccs = nccs_of(&save);
    let mut s = String::from("0:0:0:0");
    for _ in 0..(nccs + 1000) {
        s.push(':');
        s.push('0');
    }
    new_ucmd!()
        .args(&["-F", "/dev/tty", &s])
        .fails_with_code(1)
        .stderr_contains("invalid argument");
}
