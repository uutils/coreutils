/// Test if the program is running under WSL
// ref: <https://github.com/microsoft/WSL/issues/4555> @@ <https://archive.is/dP0bz>

// spell-checker:ignore (path) osrelease

pub fn is_wsl_1() -> bool {
    #[cfg(target_os = "linux")]
    {
        if is_wsl_2() {
            return false;
        }
        if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease") {
            if let Ok(s) = std::str::from_utf8(&b) {
                let a = s.to_ascii_lowercase();
                return a.contains("microsoft") || a.contains("wsl");
            }
        }
    }
    false
}

pub fn is_wsl_2() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease") {
            if let Ok(s) = std::str::from_utf8(&b) {
                let a = s.to_ascii_lowercase();
                return a.contains("wsl2");
            }
        }
    }
    false
}
