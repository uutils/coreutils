extern crate libc;

use std::io::Write;
use std::path::Path;
use std::fs;

/// Takes a user-supplied string and tries to parse to u16 mode bitmask.
pub fn parse(mode_string: &str, considering_dir: bool) -> Result<u32, String> {
    let numbers: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    // Passing 000 as the existing permissions seems to mirror GNU behaviour.
    if mode_string.contains(numbers) {
        chmod_rs::parse_numeric(0, mode_string)
    } else {
        chmod_rs::parse_symbolic(0, mode_string, considering_dir)
    }
}

/// chmod a file or directory on UNIX.
///
/// Adapted from mkdir.rs.  Handles own error printing.
///
#[cfg(any(unix, target_os = "redox"))]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|err| {
        show_info!("{}: chmod failed with error {}", path.display(), err);
    })
}

/// chmod a file or directory on Windows.
///
/// Adapted from mkdir.rs.
///
#[cfg(windows)]
pub fn chmod(path: &Path, mode: u32) -> Result<(), ()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

/// Parsing functions taken from chmod.rs.
///
/// We keep these in a dedicated module to minimize debt of duplicated code.
///
mod chmod_rs {
    extern crate libc;

    pub fn parse_numeric(fperm: u32, mut mode: &str) -> Result<u32, String> {
        let (op, pos) = try!(parse_op(mode, Some('=')));
        mode = mode[pos..].trim_left_matches('0');
        if mode.len() > 4 {
            Err(format!("mode is too large ({} > 7777)", mode))
        } else {
            match u32::from_str_radix(mode, 8) {
                Ok(change) => {
                    Ok(match op {
                        '+' => fperm | change,
                        '-' => fperm & !change,
                        '=' => change,
                        _ => unreachable!()
                    })
                }
                Err(_) => Err(String::from("numeric parsing error"))
            }
        }
    }

    pub fn parse_symbolic(mut fperm: u32, mut mode: &str, considering_dir: bool) -> Result<u32, String> {
        #[cfg(unix)]
        use libc::umask;

        #[cfg(target_os = "redox")]
        unsafe fn umask(_mask: u32) -> u32 {
            // XXX Redox does not currently have umask
            0
        }

        let (mask, pos) = parse_levels(mode);
        if pos == mode.len() {
            return Err(format!("invalid mode ({})", mode));
        }
        let respect_umask = pos == 0;
        let last_umask = unsafe {
            umask(0)
        };
        mode = &mode[pos..];
        while mode.len() > 0 {
            let (op, pos) = try!(parse_op(mode, None));
            mode = &mode[pos..];
            let (mut srwx, pos) = parse_change(mode, fperm, considering_dir);
            if respect_umask {
                srwx &= !(last_umask as u32);
            }
            mode = &mode[pos..];
            match op {
                '+' => fperm |= srwx & mask,
                '-' => fperm &= !(srwx & mask),
                '=' => fperm = (fperm & !mask) | (srwx & mask),
                _ => unreachable!()
            }
        }
        unsafe {
            umask(last_umask);
        }
        Ok(fperm)
    }

    fn parse_levels(mode: &str) -> (u32, usize) {
        let mut mask = 0;
        let mut pos = 0;
        for ch in mode.chars() {
            mask |= match ch {
                'u' => 0o7700,
                'g' => 0o7070,
                'o' => 0o7007,
                'a' => 0o7777,
                _ => break
            };
            pos += 1;
        }
        if pos == 0 {
            mask = 0o7777;  // default to 'a'
        }
        (mask, pos)
    }

    fn parse_op(mode: &str, default: Option<char>) -> Result<(char, usize), String> {
        match mode.chars().next() {
            Some(ch) => match ch {
                '+' | '-' | '=' => Ok((ch, 1)),
                _ => match default {
                    Some(ch) => Ok((ch, 0)),
                    None => Err(format!("invalid operator (expected +, -, or =, but found {})", ch))
                }
            },
            None => Err("unexpected end of mode".to_owned())
        }
    }

    fn parse_change(mode: &str, fperm: u32, considering_dir: bool) -> (u32, usize) {
        let mut srwx = fperm & 0o7000;
        let mut pos = 0;
        for ch in mode.chars() {
            match ch {
                'r' => srwx |= 0o444,
                'w' => srwx |= 0o222,
                'x' => srwx |= 0o111,
                'X' => {
                    if considering_dir || (fperm & 0o0111) != 0 {
                        srwx |= 0o111
                    }
                }
                's' => srwx |= 0o4000 | 0o2000,
                't' => srwx |= 0o1000,
                'u' => srwx = (fperm & 0o700) | ((fperm >> 3) & 0o070) | ((fperm >> 6) & 0o007),
                'g' => srwx = ((fperm << 3) & 0o700) | (fperm & 0o070) | ((fperm >> 3) & 0o007),
                'o' => srwx = ((fperm << 6) & 0o700) | ((fperm << 3) & 0o070) | (fperm & 0o007),
                _ => break
            };
            pos += 1;
        }
        if pos == 0 {
            srwx = 0;
        }
        (srwx, pos)
    }
}
