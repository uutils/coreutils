// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to parse modes

// spell-checker:ignore (vars) fperm srwx

use libc::{S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, mode_t, umask};

pub fn parse_numeric(fperm: u32, mut mode: &str, considering_dir: bool) -> Result<u32, String> {
    let (op, pos) = parse_op(mode).map_or_else(|_| (None, 0), |(op, pos)| (Some(op), pos));
    mode = mode[pos..].trim();
    let change = if mode.is_empty() {
        0
    } else {
        u32::from_str_radix(mode, 8).map_err(|e| e.to_string())?
    };
    if change > 0o7777 {
        Err(format!("mode is too large ({change} > 7777"))
    } else {
        Ok(match op {
            Some('+') => fperm | change,
            Some('-') => fperm & !change,
            // If this is a directory, we keep the setgid and setuid bits,
            // unless the mode contains 5 or more octal digits or the mode is "="
            None if considering_dir && mode.len() < 5 => change | (fperm & (0o4000 | 0o2000)),
            None | Some('=') => change,
            Some(_) => unreachable!(),
        })
    }
}

pub fn parse_symbolic(
    mut fperm: u32,
    mut mode: &str,
    umask: u32,
    considering_dir: bool,
) -> Result<u32, String> {
    let (mask, pos) = parse_levels(mode);
    if pos == mode.len() {
        return Err(format!("invalid mode ({mode})"));
    }
    let respect_umask = pos == 0;
    mode = &mode[pos..];
    while !mode.is_empty() {
        let (op, pos) = parse_op(mode)?;
        mode = &mode[pos..];
        let (mut srwx, pos) = parse_change(mode, fperm, considering_dir);
        if respect_umask {
            srwx &= !umask;
        }
        mode = &mode[pos..];
        match op {
            '+' => fperm |= srwx & mask,
            '-' => fperm &= !(srwx & mask),
            '=' => {
                if considering_dir {
                    // keep the setgid and setuid bits for directories
                    srwx |= fperm & (0o4000 | 0o2000);
                }
                fperm = (fperm & !mask) | (srwx & mask);
            }
            _ => unreachable!(),
        }
    }
    Ok(fperm)
}

fn parse_levels(mode: &str) -> (u32, usize) {
    let mut mask = 0;
    let mut pos = 0;
    for ch in mode.chars() {
        mask |= match ch {
            'u' => 0o4700,
            'g' => 0o2070,
            'o' => 0o1007,
            'a' => 0o7777,
            _ => break,
        };
        pos += 1;
    }
    if pos == 0 {
        mask = 0o7777; // default to 'a'
    }
    (mask, pos)
}

fn parse_op(mode: &str) -> Result<(char, usize), String> {
    let ch = mode
        .chars()
        .next()
        .ok_or_else(|| "unexpected end of mode".to_owned())?;
    match ch {
        '+' | '-' | '=' => Ok((ch, 1)),
        _ => Err(format!(
            "invalid operator (expected +, -, or =, but found {ch})"
        )),
    }
}

fn parse_change(mode: &str, fperm: u32, considering_dir: bool) -> (u32, usize) {
    let mut srwx = 0;
    let mut pos = 0;
    for ch in mode.chars() {
        match ch {
            'r' => srwx |= 0o444,
            'w' => srwx |= 0o222,
            'x' => srwx |= 0o111,
            'X' => {
                if considering_dir || (fperm & 0o0111) != 0 {
                    srwx |= 0o111;
                }
            }
            's' => srwx |= 0o4000 | 0o2000,
            't' => srwx |= 0o1000,
            'u' => srwx = (fperm & 0o700) | ((fperm >> 3) & 0o070) | ((fperm >> 6) & 0o007),
            'g' => srwx = ((fperm << 3) & 0o700) | (fperm & 0o070) | ((fperm >> 3) & 0o007),
            'o' => srwx = ((fperm << 6) & 0o700) | ((fperm << 3) & 0o070) | (fperm & 0o007),
            _ => break,
        }
        if ch == 'u' || ch == 'g' || ch == 'o' {
            // symbolic modes only allows perms to be a single letter of 'ugo'
            // therefore this must either be the first char or it is unexpected
            if pos != 0 {
                break;
            }
            pos = 1;
            break;
        }
        pos += 1;
    }
    if pos == 0 {
        srwx = 0;
    }
    (srwx, pos)
}

#[allow(clippy::unnecessary_cast)]
pub fn parse_mode(mode: &str) -> Result<mode_t, String> {
    #[cfg(all(
        not(target_os = "freebsd"),
        not(target_vendor = "apple"),
        not(target_os = "android")
    ))]
    let fperm = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
    #[cfg(any(target_os = "freebsd", target_vendor = "apple", target_os = "android"))]
    let fperm = (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH) as u32;

    let result = if mode.chars().any(|c| c.is_ascii_digit()) {
        parse_numeric(fperm as u32, mode, true)
    } else {
        parse_symbolic(fperm as u32, mode, get_umask(), true)
    };
    result.map(|mode| mode as mode_t)
}

pub fn get_umask() -> u32 {
    // There's no portable way to read the umask without changing it.
    // We have to replace it and then quickly set it back, hopefully before
    // some other thread is affected.
    // On modern Linux kernels the current umask could instead be read
    // from /proc/self/status. But that's a lot of work.
    // SAFETY: umask always succeeds and doesn't operate on memory. Races are
    // possible but it can't violate Rust's guarantees.
    let mask = unsafe { umask(0) };
    unsafe { umask(mask) };
    #[cfg(all(
        not(target_os = "freebsd"),
        not(target_vendor = "apple"),
        not(target_os = "android"),
        not(target_os = "redox")
    ))]
    return mask;
    #[cfg(any(
        target_os = "freebsd",
        target_vendor = "apple",
        target_os = "android",
        target_os = "redox"
    ))]
    return mask as u32;
}

#[cfg(test)]
mod test {

    #[test]
    fn symbolic_modes() {
        assert_eq!(super::parse_mode("u+x").unwrap(), 0o766);
        assert_eq!(
            super::parse_mode("+x").unwrap(),
            if crate::os::is_wsl_1() { 0o776 } else { 0o777 }
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
