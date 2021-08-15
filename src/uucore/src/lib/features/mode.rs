// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) fperm srwx

use libc::{mode_t, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};

pub fn parse_numeric(fperm: u32, mut mode: &str) -> Result<u32, String> {
    let (op, pos) = parse_op(mode, Some('='))?;
    mode = mode[pos..].trim();
    match u32::from_str_radix(mode, 8) {
        Ok(change) => {
            if change > 0o7777 {
                Err(format!("mode is too large ({} > 7777", mode))
            } else {
                Ok(match op {
                    '+' => fperm | change,
                    '-' => fperm & !change,
                    '=' => change,
                    _ => unreachable!(),
                })
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub fn parse_symbolic(
    mut fperm: u32,
    mut mode: &str,
    considering_dir: bool,
) -> Result<u32, String> {
    #[cfg(unix)]
    use libc::umask;

    let (mask, pos) = parse_levels(mode);
    if pos == mode.len() {
        return Err(format!("invalid mode ({})", mode));
    }
    let respect_umask = pos == 0;
    let last_umask = unsafe { umask(0) };
    mode = &mode[pos..];
    while !mode.is_empty() {
        let (op, pos) = parse_op(mode, None)?;
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
            _ => unreachable!(),
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
            _ => break,
        };
        pos += 1;
    }
    if pos == 0 {
        mask = 0o7777; // default to 'a'
    }
    (mask, pos)
}

fn parse_op(mode: &str, default: Option<char>) -> Result<(char, usize), String> {
    let ch = mode
        .chars()
        .next()
        .ok_or_else(|| "unexpected end of mode".to_owned())?;
    Ok(match ch {
        '+' | '-' | '=' => (ch, 1),
        _ => {
            let ch = default.ok_or_else(|| {
                format!("invalid operator (expected +, -, or =, but found {})", ch)
            })?;
            (ch, 0)
        }
    })
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
            _ => break,
        };
        pos += 1;
    }
    if pos == 0 {
        srwx = 0;
    }
    (srwx, pos)
}

pub fn parse_mode(mode: &str) -> Result<mode_t, String> {
    let fperm = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
    let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let result = if mode.contains(arr) {
        parse_numeric(fperm as u32, mode)
    } else {
        parse_symbolic(fperm as u32, mode, true)
    };
    result.map(|mode| mode as mode_t)
}

#[cfg(test)]
mod test {

    #[test]
    fn symbolic_modes() {
        assert_eq!(super::parse_mode("u+x").unwrap(), 0o766);
        assert_eq!(
            super::parse_mode("+x").unwrap(),
            if !crate::os::is_wsl_1() { 0o777 } else { 0o776 }
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
