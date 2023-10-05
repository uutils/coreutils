// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (path) osrelease

use libc::{mode_t, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};

use uucore::mode;

pub const MODE_RW_UGO: mode_t = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;

pub fn parse_mode(mode: &str) -> Result<mode_t, String> {
    let result = if mode.chars().any(|c| c.is_ascii_digit()) {
        mode::parse_numeric(MODE_RW_UGO as u32, mode)
    } else {
        mode::parse_symbolic(MODE_RW_UGO as u32, mode, true)
    };
    result.map(|mode| mode as mode_t)
}

#[cfg(test)]
mod test {
    /// Test if the program is running under WSL
    // ref: <https://github.com/microsoft/WSL/issues/4555> @@ <https://archive.is/dP0bz>
    // ToDO: test on WSL2 which likely doesn't need special handling; plan change to `is_wsl_1()` if WSL2 is less needy
    pub fn is_wsl() -> bool {
        #[cfg(target_os = "linux")]
        {
            if let Ok(b) = std::fs::read("/proc/sys/kernel/osrelease") {
                if let Ok(s) = std::str::from_utf8(&b) {
                    let a = s.to_ascii_lowercase();
                    return a.contains("microsoft") || a.contains("wsl");
                }
            }
        }
        false
    }

    #[test]
    fn symbolic_modes() {
        assert_eq!(super::parse_mode("u+x").unwrap(), 0o766);
        assert_eq!(
            super::parse_mode("+x").unwrap(),
            if is_wsl() { 0o776 } else { 0o777 }
        );
        assert_eq!(super::parse_mode("a-w").unwrap(), 0o444);
        assert_eq!(super::parse_mode("g-r").unwrap(), 0o626);
    }

    #[test]
    fn numeric_modes() {
        assert_eq!(super::parse_mode("644").unwrap(), 0o644);
        assert_eq!(super::parse_mode("+100").unwrap(), 0o766);
        assert_eq!(super::parse_mode("-4").unwrap(), 0o662);
    }
}
