extern crate libc;
use libc::{mode_t, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR};

use uucore::mode;

pub fn parse_mode(mode: Option<String>) -> Result<mode_t, String> {
    let fperm = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH | S_IWOTH;
    if let Some(mode) = mode {
        let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let result = if mode.contains(arr) {
            mode::parse_numeric(fperm as u32, mode.as_str())
        } else {
            mode::parse_symbolic(fperm as u32, mode.as_str(), true)
        };
        result.map(|mode| mode as mode_t)
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
