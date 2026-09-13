// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (path) osrelease

//! Test if the program is running under WSL
//! ref: <https://github.com/microsoft/WSL/issues/4555> @@ <https://archive.is/dP0bz>

/// Test if the program is running under WSL version 1
pub fn is_wsl_1() -> bool {
    #[cfg(target_os = "linux")]
    return !is_wsl_2()
        && std::fs::read_to_string("/proc/sys/kernel/osrelease").is_ok_and(|s| {
            let a = s.to_ascii_lowercase();
            a.contains("microsoft") || a.contains("wsl")
        });
    #[cfg(not(target_os = "linux"))]
    false
}

/// Test if the program is running under WSL version 2
pub fn is_wsl_2() -> bool {
    #[cfg(target_os = "linux")]
    return std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .is_ok_and(|s| s.to_ascii_lowercase().contains("wsl2"));
    #[cfg(not(target_os = "linux"))]
    false
}

/// Test if the program is running under WSL
pub fn is_wsl() -> bool {
    is_wsl_1() || is_wsl_2()
}
