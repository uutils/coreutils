extern crate libc;
use libc::{mode_t, S_IRGRP, S_IWGRP, S_IROTH, S_IWOTH, S_IRUSR, S_IWUSR};

fn parse_change(mode: &str, fperm: mode_t) -> (mode_t, usize) {
    let mut srwx = fperm & 0o7000;
    let mut pos = 0;
    for ch in mode.chars() {
        match ch {
            'r' => srwx |= 0o444,
            'w' => srwx |= 0o222,
            'x' => srwx |= 0o111,
            'X' => srwx |= 0o111,
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

fn parse_levels(mode: &str) -> (mode_t, usize) {
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
        mask = 0o7777;  // default to 'a'
    }
    (mask, pos)
}

fn parse_symbolic(mut fperm: mode_t, mut mode: &str) -> Result<mode_t, String> {
    let (mask, pos) = parse_levels(mode);
    if pos == mode.len() {
        return Err("invalid mode".to_owned());
    }
    mode = &mode[pos..];
    while mode.len() > 0 {
        let (op, pos) = try!(parse_op(mode, None));
        mode = &mode[pos..];
        let (srwx, pos) = parse_change(mode, fperm);
        mode = &mode[pos..];
        match op {
            '+' => fperm |= srwx & mask,
            '-' => fperm &= !(srwx & mask),
            '=' => fperm = (fperm & !mask) | (srwx & mask),
            _ => unreachable!(),
        }
    }
    Ok(fperm)
}

fn parse_op(mode: &str, default: Option<char>) -> Result<(char, usize), String> {
    match mode.chars().next() {
        Some(ch) => {
            match ch {
                '+' | '-' | '=' => Ok((ch, 1)),
                _ => {
                    match default {
                        Some(ch) => Ok((ch, 0)),
                        None => {
                            Err(format!("invalid operator (expected +, -, or =, but found {})", ch))
                        }
                    }
                }
            }
        }
        None => Err("unexpected end of mode".to_owned()),
    }
}

fn parse_numeric(fperm: mode_t, mut mode: &str) -> Result<mode_t, String> {
    let (op, pos) = try!(parse_op(mode, Some('=')));
    mode = mode[pos..].trim_left_matches('0');
    match mode_t::from_str_radix(mode, 8) {
        Ok(change) => {
            let after = match op {
                '+' => fperm | change,
                '-' => fperm & !change,
                '=' => change,
                _ => unreachable!(),
            };
            if after > 0o7777 {
                return Err("invalid mode".to_owned());
            }
            Ok(after)
        }
        Err(_) => Err("invalid mode".to_owned()),
    }
}
pub fn parse_mode(mode: Option<String>) -> Result<mode_t, String> {
    let fperm = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
    if let Some(mode) = mode {
        let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let result = if mode.contains(arr) {
            parse_numeric(fperm, mode.as_str())
        } else {
            parse_symbolic(fperm, mode.as_str())
        };
        result
    } else {
        Ok(fperm)
    }
}


#[test]
fn symbolic_modes() {
    assert_eq!(parse_mode(Some("u+x".to_owned())).unwrap(), 0o766);
    assert_eq!(parse_mode(Some("+x".to_owned())).unwrap(), 0o777);
    assert_eq!(parse_mode(Some("a-w".to_owned())).unwrap(), 0o444);
    assert_eq!(parse_mode(Some("g-r".to_owned())).unwrap(), 0o626);
}

#[test]
fn numeric_modes() {
    assert_eq!(parse_mode(Some("644".to_owned())).unwrap(), 0o644);
    assert_eq!(parse_mode(Some("+100".to_owned())).unwrap(), 0o766);
    assert_eq!(parse_mode(Some("-4".to_owned())).unwrap(), 0o662);
    assert_eq!(parse_mode(None).unwrap(), 0o666);
}
