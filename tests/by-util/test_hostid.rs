use crate::common::util::*;
extern crate regex;
use self::regex::Regex;

#[test]
fn test_normal() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();

    assert!(result.success);
    let re = Regex::new(r"^[0-9a-f]{8}").unwrap();
    assert!(re.is_match(&result.stdout.trim()));
}
