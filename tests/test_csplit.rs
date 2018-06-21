extern crate glob;

use self::glob::glob;
use common::util::*;

/// Returns a string of numbers with the given range, each on a new line.
fn generate(from: u32, to: u32) -> String {
    (from..to).fold(String::new(), |acc, v| format!("{}{}\n", acc, v))
}

#[test]
fn test_up_to_line() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10"])
        .succeeds()
        .stdout_only("18\n123\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(1, 10));
    assert_eq!(at.read("xx1"), generate(10, 51));
}

#[test]
fn test_up_to_line_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "{2}"])
        .succeeds()
        .stdout_only("18\n30\n30\n63\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx0"), generate(1, 10));
    assert_eq!(at.read("xx1"), generate(10, 20));
    assert_eq!(at.read("xx2"), generate(20, 30));
    assert_eq!(at.read("xx3"), generate(30, 51));
}

#[test]
fn test_up_to_line_sequence() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "10", "25"])
        .succeeds()
        .stdout_only("18\n45\n78\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx0"), generate(1, 10));
    assert_eq!(at.read("xx1"), generate(10, 25));
    assert_eq!(at.read("xx2"), generate(25, 51));
}

#[test]
fn test_up_to_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/"])
        .succeeds()
        .stdout_only("16\n125\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(1, 9));
    assert_eq!(at.read("xx1"), generate(9, 51));
}

#[test]
fn test_up_to_match_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{2}"])
        .succeeds()
        .stdout_only("16\n29\n30\n66\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx0"), generate(1, 9));
    assert_eq!(at.read("xx1"), generate(9, 19));
    assert_eq!(at.read("xx2"), generate(19, 29));
    assert_eq!(at.read("xx3"), generate(29, 51));
}

#[test]
fn test_up_to_match_sequence() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "/5$/"])
        .succeeds()
        .stdout_only("16\n17\n108\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx0"), generate(1, 9));
    assert_eq!(at.read("xx1"), generate(9, 15));
    assert_eq!(at.read("xx2"), generate(15, 51));
}

#[test]
fn test_up_to_match_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/+3"])
        .succeeds()
        .stdout_only("24\n117\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(1, 12));
    assert_eq!(at.read("xx1"), generate(12, 51));
}

#[test]
fn test_up_to_match_offset_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/+3", "{2}"])
        .succeeds()
        .stdout_only("24\n30\n30\n57\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx0"), generate(1, 12));
    assert_eq!(at.read("xx1"), generate(12, 22));
    assert_eq!(at.read("xx2"), generate(22, 32));
    assert_eq!(at.read("xx3"), generate(32, 51));
}

#[test]
fn test_up_to_match_negative_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/-3"])
        .succeeds()
        .stdout_only("10\n131\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(1, 6));
    assert_eq!(at.read("xx1"), generate(6, 51));
}

#[test]
fn test_up_to_match_negative_offset_repeat_twice() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/-3", "{2}"])
        .succeeds()
        .stdout_only("10\n26\n30\n75\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 4);
    assert_eq!(at.read("xx0"), generate(1, 6));
    assert_eq!(at.read("xx1"), generate(6, 16));
    assert_eq!(at.read("xx2"), generate(16, 26));
    assert_eq!(at.read("xx3"), generate(26, 51));
}

#[test]
fn test_up_to_match_repeat_always() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "/9$/", "{*}"])
        .succeeds()
        .stdout_only("16\n29\n30\n30\n30\n6\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 6);
    assert_eq!(at.read("xx0"), generate(1, 9));
    assert_eq!(at.read("xx1"), generate(9, 19));
    assert_eq!(at.read("xx2"), generate(19, 29));
    assert_eq!(at.read("xx3"), generate(29, 39));
    assert_eq!(at.read("xx4"), generate(39, 49));
    assert_eq!(at.read("xx5"), generate(49, 51));
}

#[test]
fn test_skip_to_match() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%23%"])
        .succeeds()
        .stdout_only("84\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx0"), generate(23, 51));
}

#[test]
fn test_skip_to_match_sequence1() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "%^4%"])
        .succeeds()
        .stdout_only("33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx0"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence2() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{1}", "%^4%"])
        .succeeds()
        .stdout_only("33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx0"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence3() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{1}", "/^4/"])
        .succeeds()
        .stdout_only("60\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(20, 40));
    assert_eq!(at.read("xx1"), generate(40, 51));
}

#[test]
fn test_skip_to_match_sequence4() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "/^4/"])
        .succeeds()
        .stdout_only("90\n33\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 2);
    assert_eq!(at.read("xx0"), generate(10, 40));
    assert_eq!(at.read("xx1"), generate(40, 51));
}

#[test]
fn test_skip_to_match_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%23%+3"])
        .succeeds()
        .stdout_only("75\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx0"), generate(26, 51));
}

#[test]
fn test_skip_to_match_negative_offset() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%23%-3"])
        .succeeds()
        .stdout_only("93\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 1);
    assert_eq!(at.read("xx0"), generate(20, 51));
}

#[test]
fn test_skip_to_match_repeat_always() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "%0$%", "{*}"])
        .succeeds()
        .no_stdout();

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_mix() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["numbers50.txt", "13", "%25%", "/0$/"])
        .succeeds()
        .stdout_only("27\n15\n63\n");

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx0"), generate(1, 13));
    assert_eq!(at.read("xx1"), generate(25, 30));
    assert_eq!(at.read("xx2"), generate(30, 51));
}

#[test]
fn test_option_quiet() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["--quiet", "numbers50.txt", "13", "%25%", "/0$/"])
        .succeeds()
        .no_stdout();

    let count = glob(&at.plus_as_string("xx*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("xx0"), generate(1, 13));
    assert_eq!(at.read("xx1"), generate(25, 30));
    assert_eq!(at.read("xx2"), generate(30, 51));
}

#[test]
fn test_option_prefix() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["--prefix", "dog", "numbers50.txt", "13", "%25%", "/0$/"])
        .succeeds()
        .stdout_only("27\n15\n63\n");

    let count = glob(&at.plus_as_string("dog*"))
        .expect("there should be splits created")
        .count();
    assert_eq!(count, 3);
    assert_eq!(at.read("dog0"), generate(1, 13));
    assert_eq!(at.read("dog1"), generate(25, 30));
    assert_eq!(at.read("dog2"), generate(30, 51));
}
