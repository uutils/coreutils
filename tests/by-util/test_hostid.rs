use crate::common::util::*;
extern crate regex;
use self::regex::Regex;

#[test]
fn test_normal() {
    let re = Regex::new(r"^[0-9a-f]{8}").unwrap();
    new_ucmd!().succeeds().stdout_matches(&re);
}
